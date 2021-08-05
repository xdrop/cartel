import calendar
import tempfile
import time
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
    def __init__(
        self, pre_cmd_fn=None, post_cmd_fn=None, block=False, msg="pass"
    ):
        self.tf = tempfile.NamedTemporaryFile()
        self.pre_cmd_fn = pre_cmd_fn
        self.post_cmd_fn = post_cmd_fn
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
        pre_cmd = f"{self.pre_cmd_fn(self.path)};" if self.pre_cmd_fn else ""
        post_cmd = self.post_cmd_fn(self.path) if self.post_cmd_fn else ""
        path = self.path
        msg = self.msg
        block = "blocked" if self.block else "unblocked"

        # Checks if file is empty, if it is it writes 1 to it, otherwise
        # increments and stores the incremented number
        return f"{pre_cmd}{shim_exe_path()} {block} {path} {msg}; {post_cmd}"

    @property
    def command(self):
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
    def command(self):
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
    def command(self):
        return ["bash", "-c", self.shell]


class NetListenerShim:
    def __init__(self, port, delay=0):
        self.port = port
        self.delay = delay

    @property
    def shell(self):
        return f"sleep {self.delay}; {shim_exe_path()} http {self.port}"

    @property
    def command(self):
        return ["bash", "-c", self.shell]


class EventualExitShim:
    def __init__(self, delay=0):
        self.delay = delay
        self.tf = tempfile.NamedTemporaryFile()
        epoch = calendar.timegm(time.gmtime())
        self.tf.write(str(epoch).encode("utf-8"))
        self.tf.flush()

    @property
    def shell(self):
        return f"{shim_exe_path()} eventual_exit {self.tf.name} {self.delay}"

    @property
    def command(self):
        return [
            str(shim_exe_path()),
            "eventual_exit",
            self.tf.name,
            str(self.delay),
        ]


class ExitToggleShim:
    def __init__(self):
        self.tf = tempfile.NamedTemporaryFile()
        self.exit_code = 1
        self.tf.write("1".encode("utf-8"))
        self.tf.flush()

    def toggle(self):
        self.exit_code = 0 if self.exit_code == 1 else 1
        self.tf.truncate(0)
        self.tf.write(str(self.exit_code).encode("utf-8"))
        self.tf.flush()

    @property
    def shell(self):
        return f"exit $(cat {self.tf.name})"

    @property
    def command(self):
        return ["bash", "-c", self.shell]


def _exit_cmd(exit_code):
    def _exit(*args):
        return f"echo exiting; exit {exit_code}"

    return _exit


def _delay_cmd(seconds, exit_code=0):
    def _delay(*args):
        if exit_code != 0:
            return f"sleep {seconds}; exit {exit_code}"
        return f"sleep {seconds}"

    return _delay


def service_shim(exit_code=0, delay=None, msg="pass"):
    if delay:
        return SimpleShim(
            pre_cmd_fn=_delay_cmd(delay, exit_code=exit_code), msg=msg
        )
    if exit_code != 0:
        return SimpleShim(pre_cmd_fn=_exit_cmd(exit_code), msg=msg)
    return SimpleShim(block=True, msg=msg)


def net_listener_service_shim(delay=0, port=23781):
    return NetListenerShim(port=port, delay=delay)


def env_shim():
    return EnvVarShim()


def log_file_shim():
    return LogFileShim()


def working_dir_shim():
    return WorkingDirShim()


def eventual_exit_shim(delay=0):
    return EventualExitShim(delay=delay)


def exit_toggle_shim():
    return ExitToggleShim()


def task_shim(exit_code=0, delay=None, msg="pass"):
    if delay:
        return SimpleShim(
            pre_cmd_fn=_delay_cmd(delay, exit_code=exit_code), msg=msg
        )
    if exit_code is not None:
        return SimpleShim(post_cmd_fn=_exit_cmd(exit_code), msg=msg)
    return SimpleShim(msg=msg)


def check_shim(exit_code=0):
    return SimpleShim(post_cmd_fn=_exit_cmd(exit_code))
