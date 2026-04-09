import sys

def fix():
    with open('crates/duke-interpreter/src/lib.rs', 'r') as f:
        lines = f.readlines()

    for i in range(len(lines)):
        if '/// Registry of loaded classes' in lines[i]:
            # Delete doc comment lines before the include
            # We can just change them to normal comments
            for j in range(i, i+15):
                if lines[j].startswith('///'):
                    lines[j] = lines[j].replace('///', '//')
            break

    for i in range(len(lines)):
        if lines[i].startswith('#[inline]'):
            lines[i] = ""
            break

    with open('crates/duke-interpreter/src/lib.rs', 'w') as f:
        f.writelines(lines)

fix()
