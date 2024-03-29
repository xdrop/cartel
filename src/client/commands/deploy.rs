use crate::client::cli::ClientConfig;
use crate::client::commands::deployer::{
    Deployer, ModuleDeploymentPlan, ModuleToDeploy,
};
use crate::client::definitions::read_module_definitions;
use crate::client::emoji::{
    LINK, LOOKING_GLASS, SPIRAL_NOTEBOOK, SUCCESS, TEXTBOOK, VAN,
};
use crate::client::module::{
    module_names_set, remove_checks, ModuleDefinition, ModuleMarker,
};
use crate::client::validation::validate_modules_selected;
use crate::dependency::{DependencyGraph, DependencyNode};
use anyhow::Result;
use clap::ArgMatches;
use crossbeam_queue::ArrayQueue;
use crossbeam_utils::thread;
use indicatif::MultiProgress;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct DeployOptions {
    pub force_deploy: bool,
    pub skip_checks: bool,
    pub only_selected: bool,
    pub skip_readiness_checks: bool,
    pub active_envs: Vec<String>,
    pub threads: u8,
    pub wait: bool,
}

impl DeployOptions {
    pub fn from(opts: &ArgMatches) -> DeployOptions {
        let force_deploy = opts.is_present("force");
        let skip_readiness_checks = opts.is_present("skip_readiness_checks");
        let skip_checks = opts.is_present("skip_checks");
        let wait = opts.is_present("wait");
        let serial = opts.is_present("serial");

        let active_envs = if let Some(it) = opts.values_of("env") {
            it.map(String::from).collect()
        } else {
            vec![]
        };

        let threads = if serial {
            1
        } else {
            opts.value_of("threads")
                .unwrap_or("4")
                .parse::<u8>()
                .unwrap_or(4)
        };

        let only_selected = opts.is_present("only_selected");
        Self {
            force_deploy,
            skip_checks,
            only_selected,
            skip_readiness_checks,
            active_envs,
            threads,
            wait,
        }
    }
}

pub fn deploy_cmd(
    modules_to_deploy: Vec<&str>,
    cfg: &ClientConfig,
    deploy_opts: &DeployOptions,
) -> Result<()> {
    tprintstep!("Looking for module definitions...", 1, 6, LOOKING_GLASS);
    let mut module_defs = read_module_definitions(cfg)?;
    let checks_map = remove_checks(&mut module_defs);
    let module_names = module_names_set(&module_defs);

    validate_modules_selected(&module_names, &modules_to_deploy)?;

    let deployed: Vec<_> = if !deploy_opts.only_selected {
        tprintstep!("Resolving dependencies...", 2, 6, LINK);
        let graph = DependencyGraph::from(&module_defs, &modules_to_deploy);
        let dependencies = resolve_dependencies(&graph)?;

        if deploy_opts.skip_checks {
            tprintskipped!("Running checks...", 3, 6, TEXTBOOK);
        } else {
            tprintstep!("Running checks...", 3, 6, TEXTBOOK);
            Deployer::run_checks(checks_map, &dependencies.all)?;
        }

        tprintstep!("Obtaining plan...", 4, 6, SPIRAL_NOTEBOOK);
        let deployment_plan =
            Deployer::obtain_plan(&dependencies.all, cfg, deploy_opts)?;
        tprintstep!("Deploying...", 5, 6, VAN);
        deploy_with_dependencies(
            &dependencies.groupped,
            deployment_plan,
            cfg,
            deploy_opts,
        )?;
        dependencies.all.iter().map(|d| d.key.clone()).collect()
    } else {
        tprintskipped!("Resolving dependencies...", 2, 6, LINK);
        let modules_to_deploy_set: HashSet<_> =
            modules_to_deploy.iter().copied().collect();

        let selected: Vec<_> = module_defs
            .iter()
            .filter(|m| modules_to_deploy_set.contains(m.name.as_str()))
            .collect();

        let modules_to_deploy: Vec<ModuleToDeploy> =
            selected.iter().map(|m| ModuleToDeploy::from(*m)).collect();

        if deploy_opts.skip_checks {
            tprintskipped!("Running checks...", 3, 6, TEXTBOOK);
        } else {
            tprintstep!("Running checks...", 3, 6, TEXTBOOK);
            Deployer::run_checks(checks_map, &selected)?;
        }
        tprintskipped!("Obtaining plan...", 4, 6, SPIRAL_NOTEBOOK);
        tprintstep!("Deploying...", 5, 6, VAN);
        deploy_without_dependencies(&modules_to_deploy, cfg, deploy_opts)?;
        selected.iter().map(|m| m.name.clone()).collect()
    };

    let deploy_txt =
        format!("{}: {:?}", csuccess!("Deployed modules"), deployed);
    tprintstep!(deploy_txt, 6, 6, SUCCESS);
    Ok(())
}

struct DeploymentGraph<'a> {
    groupped: Vec<Vec<ModuleToDeploy<'a>>>,
    all: Vec<&'a DependencyNode<&'a ModuleDefinition, ModuleMarker>>,
}

fn resolve_dependencies<'a>(
    graph: &'a DependencyGraph<ModuleDefinition, ModuleMarker>,
) -> Result<DeploymentGraph<'a>> {
    let sort_result = graph.group_sort()?;
    let groupped: Vec<Vec<_>> = sort_result
        .groups
        .iter()
        .map(|grp| grp.iter().map(|m| ModuleToDeploy::from(*m)).collect())
        .collect();
    Ok(DeploymentGraph {
        groupped,
        all: sort_result.flat,
    })
}

fn deploy(
    modules: &[ModuleToDeploy],
    deployment_plan: Option<Arc<ModuleDeploymentPlan>>,
    cfg: &ClientConfig,
    deploy_opts: &DeployOptions,
) -> Result<()> {
    let multiprogress = Arc::new(MultiProgress::new());
    let sync_point = Arc::new(AtomicBool::new(false));

    // Maintain a queue of modules that need to be deployed. The queue
    // will contain the indices of all such modules, and threads will
    // pick modules off the queue and deploy them.
    let queue = Arc::new(ArrayQueue::new(256));
    for idx in 0..modules.len() {
        queue
            .push(idx)
            .expect("Failed to push queue, too many modules");
    }

    let result = thread::scope(|s| -> Result<(), Box<anyhow::Error>> {
        let multiprogress = &multiprogress;
        let queue = &queue;
        let modules = &modules;
        let sync_point = &sync_point;
        let deployment_plan = &deployment_plan;
        let cfg = &cfg;
        let deploy_opts = &deploy_opts;
        let mut worker_threads = vec![];

        for _ in 0..deploy_opts.threads {
            worker_threads.push(s.spawn(move |_| -> Result<()> {
                let deployer = Deployer::new(
                    multiprogress.clone(),
                    queue.clone(),
                    deployment_plan.clone(),
                );
                deployer.do_work(modules, cfg, deploy_opts)?;
                Ok(())
            }));
        }

        let multiprogress_cln = multiprogress.clone();
        let sync_point_cln = sync_point.clone();

        let progress_sync = std::thread::spawn(move || {
            // Keep calling .join on the multiprogress handle until all child
            // threads have exited. This avoids race conditions that can happen
            // if a .join happened too early and no progress bar were added. The
            // .join call is blocking so we don't expect to be busy looping on
            // this.
            while !sync_point_cln.load(Ordering::SeqCst) {
                multiprogress_cln.join().unwrap()
            }
        });

        for worker_thread in worker_threads {
            worker_thread.join().unwrap()?;
        }
        // Once all the deployer threads have finished we can set the
        // synchronization point to true, so that the above loop can finish.
        sync_point.clone().store(true, Ordering::SeqCst);
        progress_sync
            .join()
            .expect("Failed to join progress sync thread");
        Ok(())
    });

    if let Err(e) = result.unwrap() {
        return Err(*e);
    }

    Ok(())
}

fn deploy_with_dependencies(
    groups: &[Vec<ModuleToDeploy>],
    deployment_plan: ModuleDeploymentPlan,
    cfg: &ClientConfig,
    deploy_opts: &DeployOptions,
) -> Result<()> {
    let deployment_plan = Arc::new(deployment_plan);
    for group in groups {
        deploy(group, Some(Arc::clone(&deployment_plan)), cfg, deploy_opts)?;
    }
    Ok(())
}

fn deploy_without_dependencies(
    sorted: &[ModuleToDeploy],
    cfg: &ClientConfig,
    deploy_opts: &DeployOptions,
) -> Result<()> {
    deploy(sorted, None, cfg, deploy_opts)?;
    Ok(())
}
