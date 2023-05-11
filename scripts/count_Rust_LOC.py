"""
Count Rust lines of code excluding the test cases

brew install tokei
"""

import os

original_contents = dict()

def get_LOC():
    pass

filenames = os.listdir("./src")
for filename in filenames:
    if filename.endswith(".rs") and filename != "lib.rs":
        with open("./src/" + filename, "r", encoding="utf-8") as f:
            original_contents[filename] = f.read()
        with open("./src/" + filename, "w", encoding="utf-8") as f:
            content = original_contents[filename].split("#[cfg(test)]")[0]
            if filename == "union_find.rs":
                content = ""
            f.write(content)



for filename in filenames:
    if filename.endswith(".rs"):
        with open("./src/" + filename, "w", encoding="utf-8") as f:
            f.write(original_contents[filename])
