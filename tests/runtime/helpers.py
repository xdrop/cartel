import subprocess
import tempfile
import textwrap
from contextlib import nullcontext
from pathlib import Path
from time import sleep

import pexpect
import psutil
from strip_ansi import strip_ansi as strip_ansi_fn

from runtime.shim import service_shim, task_shim


def run_service(name, exit_code=0):
    svc = service_shim(exit_code=exit_code)

    definitions_file = definition(
        f"""
        kind: Service
        name: {name}
        shell: {svc.shell}
        """
    )
    client_cmd(["deploy", "-f", name], defs=definitions_file)


def run_task(name, exit_code=0):
    svc = task_shim(exit_code=exit_code)

    definitions_file = definition(
        f"""
        kind: Task
        name: {name}
        shell: {svc.shell}
        """
    )
    client_cmd(["deploy", "-f", name], defs=definitions_file)


def stop_service(name):
    definitions_file = definition(
        f"""
        kind: Service
        name: {name}
        shell: irrelevant
        """
    )
    client_cmd(["stop", name], defs=definitions_file)


def target_dir_path():
    cwd = Path(__file__).resolve()
    target_dir = cwd.parent.parent.parent.joinpath("target")
    return target_dir


def debug_binaries_path():
    target_dir = target_dir_path()
    return target_dir.joinpath("debug")


def client_cmd(
    args, defs=None, delay=0.05, timeout=1, non_tty=False, strip_ansi=True
):
    if non_tty:
        return client_cmd_nontty(
            args=args, defs=defs, timeout=timeout, delay=delay
        )
    else:
        return client_cmd_tty(
            args=args,
            defs=defs,
            timeout=timeout,
            delay=delay,
            strip_ansi=strip_ansi,
        )


def _prep_args(args, defs=None):
    debug_binaries = debug_binaries_path()
    client_path = debug_binaries.joinpath("client")
    definitions_file_arg = []
    ctx = defs or nullcontext(None)

    if defs:
        definitions_file_arg = ["-f", defs.name]

    return (str(client_path), [*definitions_file_arg, *args], ctx)


def client_cmd_tty_expect(args, pattern, defs=None, timeout=1):
    (client_path, client_args, ctx) = _prep_args(args=args, defs=defs)
    with ctx:
        p = pexpect.spawn(client_path, client_args)
        try:
            p.expect(pattern, timeout=timeout)
        except pexpect.EOF:
            return False
        except pexpect.TIMEOUT:
            return False

    return True


def client_cmd_tty(args, defs=None, delay=0.05, timeout=1, strip_ansi=True):
    (client_path, client_args, ctx) = _prep_args(args=args, defs=defs)
    out = bytearray()

    with ctx:
        p = pexpect.spawn(client_path, client_args, timeout=timeout)
        # This speeds up the tests, consider removing if it causes any issues
        p.ptyproc.delayafterclose = 0.05
        out = p.read()

    # add sufficient delay for any operations to complete
    sleep(delay)

    output = out.decode("utf-8")

    return strip_ansi_fn(output) if strip_ansi else output


def client_cmd_nontty(args, defs=None, timeout=1, delay=0.05):
    (client_path, client_args, ctx) = _prep_args(args=args, defs=defs)

    with ctx:
        try:
            p = subprocess.run(
                [client_path, *client_args],
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                timeout=timeout,
            )
        except subprocess.TimeoutExpired as err:
            return err.output.decode("utf-8")

    # add sufficient delay for any operations to complete
    sleep(delay)

    return p.stdout.decode("utf-8")


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


def definition(definitions_file):
    tf = tempfile.NamedTemporaryFile()
    tf.write(textwrap.dedent(definitions_file).encode("utf-8"))
    tf.flush()
    return tf
