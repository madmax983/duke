import os
import re

def process_file(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()

        original = content

        content = re.sub(r'(/// A generic result type for operations returning a \[`DecodeError`\]\.)\n\n(pub enum VerifyError)', r'\1\n\2', content)
        content = re.sub(r'(/// A generic result type for operations returning a \[`VerifyError`\]\.)\n\n(pub type Result<T> = std::result::Result<T, Error>;)', r'\1\n\2', content)

        # More robust approach to remove empty lines between doc comments and the items they document
        # Let's just remove the empty lines manually in python
        lines = content.split('\n')
        i = 0
        while i < len(lines) - 1:
            if lines[i].strip().startswith('///') and lines[i+1].strip() == '':
                # peek ahead to see if there's an item
                j = i + 1
                while j < len(lines) and lines[j].strip() == '':
                    j += 1

                # If we hit an item that starts with pub, #[derive, etc., remove the blank lines
                if j < len(lines) and not lines[j].strip().startswith('///'):
                    # Wait, if there are multiple blank lines, just remove the one immediately after the doc comment
                    # Let's delete the blank lines
                    del lines[i+1:j]

            i += 1

        new_content = '\n'.join(lines)
        if new_content != original:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(new_content)
            print(f"Updated {filepath}")
    except Exception as e:
        print(e)
        pass

for root, dirs, files in os.walk('.'):
    if '.git' in root or 'target' in root: continue
    for file in files:
        if file.endswith('.rs'):
            process_file(os.path.join(root, file))
