#!/usr/bin/env python3

import sys
import serial

from drawlib import Display

d = Display(serial.Serial(sys.argv[1], baudrate=115200))

d.switch_graphics()
d.reset_clip()
d.clear(0)
d.set_color([15, 7, 8, 0][::-1])
f = int(sys.argv[2])
d.set_font(f)
height = {0: 8, 1: 16, 2: 40, 3: 120, 4: 16}
width = {0: 6, 1: 8, 2: 20, 3: 60, 4: 16}
perline = 480//width[f]

if f == 0:
    perline = 32
    d.set_pos((12, 0))
    d.text('+ 0   4   8   c     10  14  18  1c')
    for i in range(0, 8):
        d.set_pos((0, (2+i)*height[f]))
        d.text(hex(32*i)[2:])
        d.set_pos((24, (2+i)*height[f]))
        d.raw_text(bytes(range(i*perline, i*perline+perline//2)))
        d.set_pos((132, (2+i)*height[f]))
        d.raw_text(bytes(range(i*perline + perline//2, i*perline+perline)))

elif f == 3:  # very large
    d.set_pos((0, 0))
    d.text('123456789')
    d.set_pos((0, 64))
    d.text('.eE%?+-')

elif f == 4:  # symbols
    for i in range(4):
        d.set_font(4)
        d.set_color([15, 7, 8, 0][::-1])
        d.set_pos((0, 2*i*height[f]))
        d.raw_text(bytes(range(i*perline, min(i*perline+perline, 256))))
        d.set_font(1)
        d.set_pos((width[f]//4, (2*i + 1)*height[f]))
        d.set_color([7, 8, 8, 0][::-1])
        d.raw_text(bytes(sum(([ch, 32] for ch in range(i*perline, min(i*perline+perline, 256))), [])))

else:
    for i in range(0, 128//height[f]):
        d.set_pos((0, i*height[f]))
        d.raw_text(bytes(range(i*perline, min(i*perline+perline, 256))))
