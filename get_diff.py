import os
with open("duke/src/diff.rs", "r") as f:
    content = f.read()
    if "dump_diff(" in content:
        print("dump_diff is called")
    if "fn dump_diff" in content:
        print("fn dump_diff is defined")
    else:
        print("fn dump_diff is NOT defined")
