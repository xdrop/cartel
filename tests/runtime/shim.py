import tempfile
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
    def __init__(self, cmd_fn):
        self.tf = tempfile.NamedTemporaryFile()
        self.cmd_fn = cmd_fn

    @property
    def path(self):
        return self.tf.name

    @property
    def shell(self):
        cmd = self.cmd_fn(self.path)
        return f"echo pass  >> {self.path}; {cmd}"

    @property
    def cmd(self):
        return ["bash", "-c", self.shell]

    def ran(self):
        path = Path(self.path)
        mtime = path.stat().st_mtime
        data = path.read_text().replace("\n", "")

        if data != "pass":
            return None

        return mtime


def _exit_cmd(exit_code):
    def _exit(*args):
        return f"echo exiting; exit {exit_code}"

    return _exit


def _service_cmd(path):
    # simulate long running process
    return "echo pass; sleep 300"


def _task_cmd(path):
    return "echo pass"


def service_shim(exit_code=0):
    if exit_code != 0:
        return SimpleShim(cmd_fn=_exit_cmd(exit_code))
    return SimpleShim(cmd_fn=_service_cmd)


def task_shim(exit_code=0):
    if exit_code != 0:
        return SimpleShim(cmd_fn=_exit_cmd(exit_code))
    return SimpleShim(cmd_fn=_task_cmd)


def check_shim(exit_code=0):
    return SimpleShim(cmd_fn=_exit_cmd(exit_code))
