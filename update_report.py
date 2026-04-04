#!/usr/bin/env python3

# Read the report script
with open('/tmp/pnl_telegram.py', 'r') as f:
    content = f.read()

# Find the ghost count section and add ghost stats
old_ghost = '''    try:
        ghost_count = int(subprocess.check_output(['grep', '-c', 'GHOST DETECTED', '/tmp/pingpong.log'], stderr=subprocess.DEVNULL).decode().strip())
    except:
        ghost_count = 0'''

new_ghost = '''    try:
        ghost_count = int(subprocess.check_output(['grep', '-c', 'GHOST DETECTED', '/tmp/pingpong.log'], stderr=subprocess.DEVNULL).decode().strip())
    except:
        ghost_count = 0
    
    # GHOST SIMULATION: Get stats from bot
    # We'll parse the log for ghost simulation results
    try:
        ghost_sim_lines = subprocess.check_output(['grep', 'GHOSTED\\|EXECUTABLE\\|PARTIAL', '/tmp/pingpong.log'], stderr=subprocess.DEVNULL).decode().strip().split('\\n')
        ghosted = sum(1 for line in ghost_sim_lines if 'GHOSTED' in line)
        executable = sum(1 for line in ghost_sim_lines if 'EXECUTABLE' in line)
        partial = sum(1 for line in ghost_sim_lines if 'PARTIAL' in line)
        total_sim = ghosted + executable + partial
        ghost_rate = (ghosted / total_sim * 100) if total_sim > 0 else 0
    except:
        ghosted = 0
        executable = 0
        partial = 0
        total_sim = 0
        ghost_rate = 0'''

content = content.replace(old_ghost, new_ghost)

# Add ghost stats to the report
old_report = '''    lines.append(f'\\n**👻 Ghosts:** {ghost_count}\\n')'''

new_report = '''    lines.append(f'\\n**👻 Ghosts:** {ghost_count}\\n')
    
    # Ghost simulation stats
    if total_sim > 0:
        lines.append(f'**🔮 Ghost Simulation (Dry Run):**\\n')
        lines.append(f'├ Total Signals: {total_sim}\\n')
        lines.append(f'├ Ghosted: {ghosted} ({ghosted/total_sim*100:.1f}%)\\n')
        lines.append(f'├ Executable: {executable} ({executable/total_sim*100:.1f}%)\\n')
        lines.append(f'└ Partial: {partial} ({partial/total_sim*100:.1f}%)\\n')
        lines.append(f'\\n**Ghost Rate:** {ghost_rate:.1f}%\\n')'''

content = content.replace(old_report, new_report)

# Write back
with open('/tmp/pnl_telegram.py', 'w') as f:
    f.write(content)

print('Updated report with ghost simulation stats')
