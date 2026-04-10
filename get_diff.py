import subprocess

out = subprocess.check_output(['git', 'diff', '--stat', 'origin/main', 'HEAD']).decode('utf-8')
print(out)
