with open('.jules/atlas.md', 'r') as f:
    lines = f.readlines()

new_lines = []
skip = False
for line in lines:
    if line.startswith('**[Encapsulate `duke_classfile` and `duke_telemetry` Facades]**'):
        skip = True
    if skip and line.strip() == '':
        skip = False
    elif not skip:
        new_lines.append(line)

with open('.jules/atlas.md', 'w') as f:
    f.writelines(new_lines)
