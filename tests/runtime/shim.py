import tempfile
from functools import cache
from os.path import realpath
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
    def process_name(self):
        return shim_exe_path().name

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


class EnvVarShim:
    def __init__(self):
        self.tf = tempfile.NamedTemporaryFile()
        self._read = False
        self._environment_vars = None

    def _update(self):
        path = Path(self.tf.name)
        data = path.read_text().splitlines()
        env = {}
        for line in data:
            parts = line.split("=")
            key, val = parts[0], parts[1]
            env[key] = val
        self._read = True
        self._environment_vars = env
        self.tf.close()

    @property
    def environment_vars(self):
        if not self._read:
            self._update()
        return self._environment_vars

    @property
    def shell(self):
        return f"printenv > {self.tf.name}"

    @property
    def cmd(self):
        return ["bash", "-c", self.shell]


class LogFileShim:
    def __init__(self):
        self.tf = tempfile.NamedTemporaryFile()
        self._read = False
        self._log_file_content = None

    def _update(self):
        path = Path(self.tf.name)
        self._read = True
        self._log_file_content = path.read_text().replace("\n", "")
        self.tf.close()

    @property
    def log_file_path(self):
        return self.tf.name

    @property
    def written_to_log_file(self):
        if not self._read:
            self._update()
        return self._log_file_content == "pass"

    @property
    def shell(self):
        return f"echo pass > {self.log_file_path}"

    @property
    def cmd(self):
        return ["bash", "-c", self.shell]


class WorkingDirShim:
    def __init__(self):
        self.tf = tempfile.NamedTemporaryFile()
        self.td = tempfile.TemporaryDirectory()
        self._read = False
        self._workdir = None

    def _update(self):
        path = Path(self.tf.name)
        self._read = True
        self._workdir = path.read_text().replace("\n", "")
        self.tf.close()
        self.td.cleanup()

    @property
    def working_dir(self):
        return self.td.name

    @property
    def ran_in_workdir(self):
        if not self._read:
            self._update()
        return realpath(self._workdir) == realpath(self.td.name)

    @property
    def shell(self):
        return f"pwd > {self.tf.name}"

    @property
    def cmd(self):
        return ["bash", "-c", self.shell]


def _exit_cmd(exit_code):
    def _exit(*args):
        return f"echo exiting; exit {exit_code}"

    return _exit


def _timeout_cmd(seconds, exit_code=0):
    def _timeout(*args):
        if exit_code != 0:
            return f"sleep {seconds}; exit {exit_code}"
        return f"sleep {seconds}"

    return _timeout


def service_shim(exit_code=0):
    if exit_code != 0:
        return SimpleShim(cmd_fn=_exit_cmd(exit_code))
    return SimpleShim(block=True, msg="pass")


def env_shim():
    return EnvVarShim()


def log_file_shim():
    return LogFileShim()


def working_dir_shim():
    return WorkingDirShim()


def task_shim(exit_code=0, timeout=None):
    if timeout:
        return SimpleShim(cmd_fn=_timeout_cmd(timeout, exit_code=exit_code))
    if exit_code != 0:
        return SimpleShim(cmd_fn=_exit_cmd(exit_code))
    return SimpleShim(msg="pass")


def check_shim(exit_code=0):
    return SimpleShim(cmd_fn=_exit_cmd(exit_code))
