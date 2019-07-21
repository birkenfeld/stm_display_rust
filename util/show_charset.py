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

for i in range(0, 128//height[f]):
    d.set_pos((0, i*height[f]))
    d.raw_text(bytes(range(i*perline, min(i*perline+perline, 256))))
