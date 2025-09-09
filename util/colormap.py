s = '\n'
for x in range(16):
    for y in range(16):
        c = y*16+x
        s += ("\x1b[48;5;%d;38;5;%dm " % (c,255-c)) + hex(c+256)[-2:] + " \x1b[0m "
print(s, end='')
