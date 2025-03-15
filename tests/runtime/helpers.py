import tempfile
import textwrap

import psutil

from runtime.client import client_cmd
from runtime.shim import service_shim, task_shim


def run_service(name, exit_code=0):
    svc = service_shim(exit_code=exit_code)

    definitions_file = definitions(
        f"""
        kind: Service
        name: {name}
        shell: {svc.shell}
        """
    )
    with definitions_file as df:
        client_cmd(["-f", df.name, "deploy", "-f", name])
    return svc


def run_task(name, exit_code=0):
    task = task_shim(exit_code=exit_code)

    definitions_file = definitions(
        f"""
        kind: Task
        name: {name}
        shell: {task.shell}
        """
    )
    with definitions_file as df:
        client_cmd(["-f", df.name, "deploy", "-f", name])
    return task


def stop_service(name):
    definitions_file = definitions(
        f"""
        kind: Service
        name: {name}
        shell: irrelevant
        """
    )
    with definitions_file as df:
        client_cmd(["-f", df.name, "stop", name])


def process_running(process_name):
    for proc in psutil.process_iter():
        try:
            # Check if process name contains the given name string.
            if process_name.lower() in proc.name().lower():
                return True
        except (
            psutil.NoSuchProcess,
            psutil.AccessDenied,
            psutil.ZombieProcess,
        ):
            pass
    return False


def kill_all_shims():
    for proc in psutil.process_iter():
        try:
            if "execshim" in proc.name().lower():
                proc.kill()
        except (
            psutil.NoSuchProcess,
            psutil.AccessDenied,
            psutil.ZombieProcess,
        ):
            pass


def find_pid(process_name, pid=None):
    for proc in psutil.process_iter():
        try:
            # Check if process name contains the given name string.
            if process_name.lower() in proc.name().lower():
                if pid and proc.pid == pid:
                    return proc.pid
                else:
                    return proc.pid
        except (
            psutil.NoSuchProcess,
            psutil.AccessDenied,
            psutil.ZombieProcess,
        ):
            pass
    return None


def definitions(definitions_file):
    tf = tempfile.NamedTemporaryFile()
    tf.write(textwrap.dedent(definitions_file).encode("utf-8"))
    tf.flush()
    return tf
