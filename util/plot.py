#!/usr/bin/env python3
import sys
import time
import serial
import random
import math

from drawlib import Display

d = Display(serial.Serial(sys.argv[1], baudrate=115200))

d.switch_graphics()
d.clear(0)

d.set_color([0, 8, 7, 15])
d.set_font(0)
d.pos_text((0, 1), "100")
d.pos_text((0, 120), "  0")
d.set_font(1)
d.pos_text((1, 58), "T2")
d.lines((20, 0), (20, 127))

y1 = 50

arr = [
    int(40 * math.sin(0.02*x) + 60 + random.random() * math.sin(0.0437*x) * 20) for x in range(479)
]

d.set_color([7]*4)
d.lines((21, 64), (479, 64))
d.set_color([11]*4)

d.plot(21, *arr[21:250])
d.plot(250, *arr[250:479])

y0 = arr[478]
while 1:
    time.sleep(1)
    d.copy_rect((22, 0), (479, 127), (21, 0))
    y1 = max(min(y0 + int(random.random() * 10 - 5), 127), 0)
    d.set_color([11]*4)
    d.lines((477, y0), (478, y1))
    y0 = y1
