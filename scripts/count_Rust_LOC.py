"""
Count Rust lines of code excluding the test cases

brew install tokei
"""

import os, sys, subprocess

original_contents = dict()

def get_LOC():
    command = "tokei src/ -e src/blossom_v.rs -e src/bin -e *.py -C".split(" ")
    stdout = subprocess.PIPE
    process = subprocess.Popen(command, universal_newlines=True, stdout=stdout, stderr=sys.stderr, bufsize=100000)
    stdout, _ = process.communicate()
    assert process.returncode == 0
    print(stdout)
    return stdout, process.returncode

get_LOC()

"""
===============================================================================
 Language            Files        Lines         Code     Comments       Blanks
===============================================================================
 Rust                   16        13495        12000          497          998
===============================================================================
 Total                  16        13495        12000          497          998
===============================================================================
"""

# # excluding test cases
# filenames = os.listdir("./src")
# for filename in filenames:
#     if filename.endswith(".rs") and filename != "lib.rs":
#         with open("./src/" + filename, "r", encoding="utf-8") as f:
#             original_contents[filename] = f.read()
#         with open("./src/" + filename, "w", encoding="utf-8") as f:
#             content = original_contents[filename].split("#[cfg(test)]")[0]
#             f.write(content)

# get_LOC()


# for filename in original_contents:
#     with open("./src/" + filename, "w", encoding="utf-8") as f:
#         f.write(original_contents[filename])
