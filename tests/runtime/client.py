import subprocess
from contextlib import nullcontext
from time import sleep

import pexpect
from strip_ansi import strip_ansi as strip_ansi_fn

from runtime.conftest import debug_binaries_path


def client_cmd(
    args, defs=None, delay=0.05, timeout=1, non_tty=False, strip_ansi=True
):
    if non_tty:
        return client_cmd_run_nontty(
            args=args, defs=defs, timeout=timeout, delay=delay
        )
    else:
        return client_cmd_run_tty(
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


class ClientTty:
    def __init__(self, args, defs):
        (client_path, client_args, ctx) = _prep_args(args=args, defs=defs)
        self.client_path = client_path
        self.client_args = client_args
        self.ctx = ctx

    def __enter__(self):
        self.ctx.__enter__()
        return self

    def __exit__(self, type, value, traceback):
        self.ctx.__exit__(type, value, traceback)

    def spawn(self):
        self.p = pexpect.spawn(self.client_path, self.client_args)

    def expect(self, pattern=pexpect.EOF, timeout=-1):
        try:
            self.p.expect(pattern, timeout=timeout)
        except pexpect.EOF:
            return False
        except pexpect.TIMEOUT:
            return False
        return True


def client_cmd_tty(args, defs=None):
    tty = ClientTty(args, defs)
    tty.spawn()
    return tty


def client_cmd_run_tty(args, defs=None, delay=0.05, timeout=1, strip_ansi=True):
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


def client_cmd_run_nontty(args, defs=None, timeout=1, delay=0.05):
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
