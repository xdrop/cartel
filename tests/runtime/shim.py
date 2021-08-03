import tempfile
from functools import cache
from pathlib import Path


@cache
def shim_exe_path():
    cwd = Path(__file__).resolve()
    return (
        cwd.parent.parent.joinpath("execshim")
        .joinpath("target")
        .joinpath("release")
        .joinpath("execshim")
    )


class SimpleShim:
    def __init__(self, cmd_fn=None, block=False, msg="pass"):
        self.tf = tempfile.NamedTemporaryFile()
        self.cmd_fn = cmd_fn
        self.msg = msg
        self.read = False
        self.block = block
        self._counter = None
        self._mtime = None
        self._signal = None

    @property
    def path(self):
        return self.tf.name

    @property
    def shell(self):
        cmd = self.cmd_fn(self.path) if self.cmd_fn else ""
        path = self.path
        msg = self.msg
        block = "blocked" if self.block else "unblocked"
        # Checks if file is empty, if it is it writes 1 to it, otherwise
        # increments and stores the incremented number
        return f"{shim_exe_path()} {block} {path} {msg}; {cmd}"

    @property
    def cmd(self):
        return ["bash", "-c", self.shell]

    def _update(self):
        path = Path(self.path)
        mtime = path.stat().st_mtime
        data = path.read_text().split("|")

        if not data or len(data) == 1:
            counter = 0
            signal = "None"
        else:
            count, signal = data[0], data[1]
            try:
                counter = int(count)
            except ValueError:
                counter = 0

        self._counter = counter
        self._signal = signal
        self._mtime = mtime
        self.read = True
        self.tf.close()

    @property
    def last_ran(self):
        if not self.read:
            self._update()

        return self._mtime

    @property
    def times_ran(self):
        if not self.read:
            self._update()

        return self._counter

    @property
    def signal(self):
        if not self.read:
            self._update()

        return self._signal

    def ran(self):
        if not self.read:
            self._update()

        return self._counter > 0

    def ran_once(self):
        if not self.read:
            self._update()

        return self._counter == 1


def _exit_cmd(exit_code):
    def _exit(*args):
        return f"echo exiting; exit {exit_code}"

    return _exit


def service_shim(exit_code=0):
    if exit_code != 0:
        return SimpleShim(cmd_fn=_exit_cmd(exit_code))
    return SimpleShim(block=True, msg="pass")


def task_shim(exit_code=0):
    if exit_code != 0:
        return SimpleShim(cmd_fn=_exit_cmd(exit_code))
    return SimpleShim(msg="pass")


def check_shim(exit_code=0):
    return SimpleShim(cmd_fn=_exit_cmd(exit_code))
