import os, sys, subprocess, importlib
git_root_dir = subprocess.run("git rev-parse --show-toplevel", cwd=os.path.dirname(os.path.abspath(__file__))
    , shell=True, check=True, capture_output=True).stdout.decode(sys.stdout.encoding).strip(" \r\n")
# useful folders
rust_dir = git_root_dir
demo_dir = os.path.join(git_root_dir, "tutorial", "src", "demo")
sys.path.insert(0, demo_dir)


def test_all_demos():
    for filename in os.listdir(demo_dir):
        if not filename.endswith(".py"):
            continue
        print("[test demo]", filename)
        module_name = filename[:-3]
        spec = importlib.util.spec_from_file_location(module_name, os.path.join(demo_dir, filename))
        module = importlib.util.module_from_spec(spec)
        sys.modules[module_name] = module
        spec.loader.exec_module(module)
