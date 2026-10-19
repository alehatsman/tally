# tally's raise on Windows. TALLY_RAISE_CMD in WSL points here
# (docs/13-bridge.md, question 2).
#
# AppActivate alone is refused by Windows' foreground lock. Measured
# 2026-10-19 from WSL, against a Windows Terminal window behind Chrome: it
# returned True every time and the window stayed behind, 0 raises of 3. A
# synthetic Alt press first makes this process the one that received the
# last input, which the lock honours: 3 raises of 4.
#
# The Alt lands on whatever had the foreground. Most programs ignore a
# bare Alt tap; a few focus their menu bar.
param([string]$Title = 'tally')
$shell = New-Object -ComObject WScript.Shell
$shell.SendKeys('%')
[void]$shell.AppActivate($Title)
