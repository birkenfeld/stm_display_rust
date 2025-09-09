#!/usr/bin/env python3

import sys
import time
import serial

from drawlib import Display

d = Display(serial.Serial(sys.argv[1], baudrate=115200))

palette = [0, 8, 7, 15]

d.reset_clip()
d.clear(0)
d.set_font(3)
d.set_color([0, 235, 240, 245])
# no colon in vlarge font... fake it with two dots
d.pos_text((130, 20), ".")
d.copy_rect((130, 60), (167, 83), (130, 40))
d.pos_text((290, 20), ".")
d.copy_rect((290, 60), (327, 83), (290, 40))

d.set_attrs(0, (30, 30), palette, 3)
d.set_attrs(1, (190, 30), palette, 3)
d.set_attrs(2, (350, 30), palette, 3)
d.switch_graphics()

while True:
    tstr = time.strftime('%H%M%S')
    d.attr_text(0, tstr[:2])
    d.attr_text(1, tstr[2:4])
    d.attr_text(2, tstr[4:6])
    time.sleep(1)
