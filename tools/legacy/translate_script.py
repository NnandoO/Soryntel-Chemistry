import os
import re

dirs_to_scan = ['src/interfaz', 'src/motores', 'src/sim', 'src/thermo', 'src/diagrams', 'src/aqueous', 'src/tools']
files_to_modify = []

for d in dirs_to_scan:
    if os.path.exists(d):
        for f in os.listdir(d):
            if f.endswith('.cpp') or f.endswith('.hpp'):
                files_to_modify.append(os.path.join(d, f))
    else:
        print(f"Skipping {d} because it does not exist")

# regex for finding functions like ImGui::Text("something", ...) and replacing string with tr("something")
functions = [
    'ImGui::Text',
    'ImGui::TextDisabled',
    'ImGui::TextColored',
    'ImGui::TextWrapped',
    'ImGui::Button',
    'ImGui::MenuItem',
    'ImGui::BeginMenu',
    'ImGui::Begin',
    'ImGui::Selectable',
    'ImGui::TreeNode',
    'ImGui::BeginTabItem',
    'ImGui::InputText',
    'ImGui::Checkbox'
]

def replace_in_file(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
    except UnicodeDecodeError:
        print(f"Error reading {filepath} with utf-8")
        return

    original_content = content
    
    for func in functions:
        # Match func("string"
        pattern = r'(' + re.escape(func) + r'\s*\(\s*)("([^"\\]|\\.)*")'
        
        def repl(match):
            prefix = match.group(1)
            string_lit = match.group(2)
            # If string is just "##something", don't translate
            if string_lit.startswith('"##'):
                return match.group(0)
            # If it's already translated, don't translate
            return prefix + 'tr(' + string_lit + ')'
            
        content = re.sub(pattern, repl, content)

    if content != original_content:
        # Add include if necessary
        if '#include "utils/i18n.h"' not in content:
            # find first include and put it there
            if '#include' in content:
                content = re.sub(r'(#include[^\n]*\n)', r'\1#include "utils/i18n.h"\n', content, count=1)
            else:
                content = '#include "utils/i18n.h"\n' + content

        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"Processed {filepath}")

for f in files_to_modify:
    replace_in_file(f)
