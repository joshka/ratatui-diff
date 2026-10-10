"""Run serialized folding measurements; call only during an exclusive timing window."""
import argparse
import subprocess
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("executable", type=Path)
parser.add_argument("output", type=Path)
args = parser.parse_args()
args.output.parent.mkdir(parents=True, exist_ok=True)
with args.output.open("w") as output:
    output.write("files,lines_per_file,phase,mode,wrap,files_closed,iterations,mean_us,folds\n")
    for files, lines in [(1, 1000), (1, 100000), (100, 1000)]:
        for mode in ["unified", "split"]:
            for wrap in ["nowrap", "wrap"]:
                for closed in ["expanded", "closed"]:
                    for phase in ["initial", "context-state", "file-state", "context-first",
                                  "file-first", "resize", "resize-open", "steady", "steady-open",
                                  "navigation"]:
                        count = 3 if phase == "initial" else 20
                        if phase in ["context-state", "file-state", "navigation"]:
                            count = 10000
                        command = [str(args.executable.resolve()), str(files), str(lines), phase,
                                   mode, wrap, closed, str(count)]
                        result = subprocess.check_output(command, text=True)
                        output.write(result)
                        output.flush()
                        print(result.strip(), flush=True)
