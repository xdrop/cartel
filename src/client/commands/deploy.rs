use crate::client::cli::ClientConfig;
use crate::client::commands::deployer::{Deployer, ModuleToDeploy};
use crate::client::config::read_module_definitions;
use crate::client::emoji::{LINK, LOOKING_GLASS, SUCCESS, VAN};
use crate::client::module::{module_names_set, remove_checks};
use crate::client::validation::validate_modules_selected;
use crate::dependency::DependencyGraph;
use anyhow::{anyhow, Result};
use clap::ArgMatches;
use crossbeam_queue::ArrayQueue;
use crossbeam_utils::thread;
use indicatif::MultiProgress;
use std::sync::atomic::Ordering;
use std::{collections::HashSet, sync::atomic::AtomicBool, sync::Arc};

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
            skip_readiness_checks,
            skip_checks,
            only_selected,
            active_envs,
            wait,
            threads,
        }
    }
}

pub fn deploy_cmd(
    modules_to_deploy: Vec<&str>,
    cfg: &ClientConfig,
    deploy_opts: &DeployOptions,
) -> Result<()> {
    tprintstep!("Looking for module definitions...", 1, 5, LOOKING_GLASS);
    let mut module_defs = read_module_definitions(&cfg)?;
    let checks_map = remove_checks(&mut module_defs);
    let module_names = module_names_set(&module_defs);

    validate_modules_selected(&module_names, &modules_to_deploy)?;

    let dep_graph: DependencyGraph<_, _>;

    let deployed: Vec<_> = if !deploy_opts.only_selected {
        tprintstep!("Resolving dependencies...", 2, 5, LINK);
        dep_graph = DependencyGraph::from(&module_defs, &modules_to_deploy);
        let sort_result = dep_graph.group_sort()?;
        let modules_to_deploy: Vec<Vec<ModuleToDeploy>> = sort_result
            .groups
            .iter()
            .map(|grp| grp.iter().map(|m| ModuleToDeploy::from(*m)).collect())
            .collect();

        Deployer::run_checks(checks_map, &sort_result.flat, deploy_opts)?;

        tprintstep!("Deploying...", 4, 5, VAN);
        deploy_with_dependencies(&modules_to_deploy, cfg, deploy_opts)?;
        sort_result.flat.iter().map(|d| &d.key).collect()
    } else {
        let msg = format!("Resolving dependencies... {}", cdim!("(Skip)"));
        tprintstep!(msg, 2, 5, LINK);
        let modules_to_deploy_set: HashSet<_> =
            modules_to_deploy.iter().copied().collect();

        let selected: Vec<_> = module_defs
            .iter()
            .filter(|m| modules_to_deploy_set.contains(m.name.as_str()))
            .collect();

        let modules_to_deploy: Vec<ModuleToDeploy> =
            selected.iter().map(|m| ModuleToDeploy::from(*m)).collect();

        Deployer::run_checks(checks_map, &selected, deploy_opts)?;
        tprintstep!("Deploying...", 4, 5, VAN);
        deploy_without_dependencies(&modules_to_deploy, cfg, deploy_opts)?;
        selected.iter().map(|m| &m.name).collect()
    };

    let deploy_txt =
        format!("{}: {:?}", csuccess!("Deployed modules"), deployed);
    tprintstep!(deploy_txt, 5, 5, SUCCESS);
    Ok(())
}

fn deploy(
    modules: &[ModuleToDeploy],
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
        let cfg = &cfg;
        let deploy_opts = &deploy_opts;
        let mut worker_threads = vec![];

        for _ in 0..deploy_opts.threads {
            worker_threads.push(s.spawn(move |_| -> Result<()> {
                let deployer =
                    Deployer::new(multiprogress.clone(), queue.clone());
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
        return Err(anyhow!(e));
    }

    Ok(())
}

fn deploy_with_dependencies(
    groups: &[Vec<ModuleToDeploy>],
    cfg: &ClientConfig,
    deploy_opts: &DeployOptions,
) -> Result<()> {
    for group in groups {
        deploy(group, cfg, deploy_opts)?;
    }
    Ok(())
}

fn deploy_without_dependencies(
    sorted: &[ModuleToDeploy],
    cfg: &ClientConfig,
    deploy_opts: &DeployOptions,
) -> Result<()> {
    deploy(sorted, cfg, deploy_opts)?;
    Ok(())
}
