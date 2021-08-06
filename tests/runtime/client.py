import subprocess
import sys
from functools import cache
from time import sleep

import pexpect
from strip_ansi import strip_ansi as strip_ansi_fn

from runtime.paths import debug_binaries_path


def client_cmd(args, delay=0.05, timeout=1, non_tty=False, strip_ansi=True):
    if non_tty:
        return client_cmd_run_nontty(args=args, timeout=timeout, delay=delay)
    else:
        return client_cmd_run_tty(
            args=args,
            timeout=timeout,
            delay=delay,
            strip_ansi=strip_ansi,
        )


@cache
def get_client_path():
    debug_binaries = debug_binaries_path()
    client_path = debug_binaries.joinpath("client")
    return str(client_path)


class ClientTty:
    def __init__(self, args):
        self.client_path = get_client_path()
        self.client_args = args

    def __enter__(self):
        return self

    def __exit__(self, type, value, traceback):
        pass

    def spawn(self):
        self.p = pexpect.spawn(
            self.client_path, self.client_args, logfile=sys.stdout.buffer
        )

    def expect(self, pattern=pexpect.EOF, timeout=-1):
        try:
            self.p.expect(pattern, timeout=timeout)
        except pexpect.EOF:
            return False
        except pexpect.TIMEOUT:
            return False
        return True


def client_cmd_tty(args):
    tty = ClientTty(args)
    tty.spawn()
    return tty


def client_cmd_run_tty(args, delay=0.05, timeout=1, strip_ansi=True):
    client_path = get_client_path()
    out = bytearray()

    p = pexpect.spawn(
        client_path, args, timeout=timeout, logfile=sys.stdout.buffer
    )
    # This speeds up the tests, consider removing if it causes any issues
    p.ptyproc.delayafterclose = 0.05
    out = p.read()

    # add sufficient delay for any operations to complete
    sleep(delay)

    output = out.decode("utf-8")

    return strip_ansi_fn(output) if strip_ansi else output


def client_cmd_run_nontty(args, timeout=1, delay=0.05):
    client_path = get_client_path()
    try:
        p = subprocess.run(
            [client_path, *args],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as err:
        return err.output.decode("utf-8")

    # add sufficient delay for any operations to complete
    sleep(delay)

    return p.stdout.decode("utf-8")
