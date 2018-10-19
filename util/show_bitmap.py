f = open('src/logo_mlz.dat', 'rb').read()
bits = 1
off = 0
for y in range(88):
    for x in range(240):
        if bits == 1:
            bits = f[off] | 0x100
            off += 1
        print(bits&3, end= ' ')
        bits >>= 2
    print()
