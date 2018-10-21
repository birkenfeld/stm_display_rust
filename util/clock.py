#!/usr/bin/env python3

import sys
import time
import serial

from drawlib import Display

d = Display(serial.Serial(sys.argv[1], baudrate=115200))

color = [0, 8, 7, 15]

d.clear(0)
d.set_font(3)
d.set_color([0, 235, 240, 245])
d.pos_text((130, 4), ":")
d.pos_text((290, 4), ":")
d.set_attrs(0, (20, 4), color, 3)
d.set_attrs(1, (180, 4), color, 3)
d.set_attrs(2, (340, 4), color, 3)
d.switch_graphics()

while True:
    tstr = time.strftime('%H%M%S')
    d.attr_text(0, tstr[:2])
    d.attr_text(1, tstr[2:4])
    d.attr_text(2, tstr[4:6])
    time.sleep(1)
