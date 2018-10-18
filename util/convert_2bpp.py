#!/usr/bin/env python3

import sys

inf, outf = sys.argv[1:]

data = open(inf, 'rb').read()
out = open(outf, 'wb')

for i in range(0, len(data), 4):
    out.write(bytes([
        (data[i] & 0x3) << 6 | (data[i+1] & 0x3) << 4 |
        (data[i+2] & 0x3) << 2 | (data[i] & 0x3)
    ]))
