#!/usr/bin/env python3

import sys
import serial

from drawlib import Display

d = Display(serial.Serial(sys.argv[1], baudrate=115200))

d.switch_graphics()
d.reset_clip()
d.clear(15)
d.set_color([15, 7, 8, 0])
f = int(sys.argv[2])
d.set_font(f)
height = {0: 8, 1: 16, 2: 40}

for i in range(0, 5):
    d.set_pos((0, i*height[f]))
    d.raw_text(bytes(range(i*60, min(i*60+60, 256))))
