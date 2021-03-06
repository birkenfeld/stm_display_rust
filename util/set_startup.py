#!/usr/bin/env python3

import drawlib
import serial
import sys

s = serial.Serial(sys.argv[1], baudrate=115200)
d = drawlib.Display(s)

d.record_startup()
d.clear(15)
d.set_pos((120, 7))
d.set_color([15, 188, 104, 60])
d.icon(0)
d.set_color([15, 7, 8, 0])
d.set_font(1)
d.pos_text((120, 103), "Touch me!".center(30))
d.switch_graphics()
d.set_startup()

d._reset()
