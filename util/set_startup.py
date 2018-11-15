#!/usr/bin/env python3

import drawlib
import serial

s = serial.Serial('/dev/ttyUSB0', baudrate=115200)
d = drawlib.Display(s)

d.record_startup()
d.clear(15)
#d.set_pos(((480-240)//2, 7))
#d.set_color([60,104,188,15])
#d.icon(0)
d.set_pos(((480-244)//2, 7))
d.set_color([0, 8, 250, 15])
d.icon(1)
d.set_color([15, 45, 21, 4])
d.set_font(1)
d.pos_text((120, 103), "Starting up...".center(30))
d.switch_graphics()
d.set_startup()

d._reset()
