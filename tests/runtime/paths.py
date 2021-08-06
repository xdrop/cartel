from pathlib import Path


def target_dir_path():
    cwd = Path(__file__).resolve()
    target_dir = cwd.parent.parent.parent.joinpath("target")
    return target_dir


def debug_binaries_path():
    target_dir = target_dir_path()
    return target_dir.joinpath("debug")
