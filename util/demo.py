#!/usr/bin/env python3
import sys
import time
import serial
import random

from drawlib import Display

d = Display(serial.Serial(sys.argv[1], baudrate=115200))

BLACK = [0, 0, 0, 0]
GRAY = [0, 239, 240, 245]
WHITE = [0, 8, 7, 15]
RED = [0, 52, 124, 196]
BLUE = [0, 0, 0, 27]
GREEN = [0, 28, 34, 46]
ALARM = [1, 197, 223, 15]

MARQUEE = ("compressor off, cooling water temperature alarm, "
           "cold head has spontaneously combusted --- ")
marq_len = len(MARQUEE)
marq_off = 0

d.reset_clip()
d.clear(15)
d.set_pos((120, 7))
d.set_color([15, 188, 104, 60])
d.image(0)
d.set_color(WHITE[::-1])
d.set_font(1)
d.pos_text((120, 103), "Booting...".center(30))
d.switch_graphics()

time.sleep(3)
d.pos_text((120, 103), "Getting network address...".center(30))
time.sleep(2)
d.pos_text((120, 103), "Starting SeCOP servers...".center(30))
time.sleep(2)
d.pos_text((120, 103), "Reticulating splines...".center(30))
time.sleep(2)
d.pos_text((120, 103), "Ready!".center(30))
time.sleep(1)

d.clear(0)
d.set_attrs(0, (21 * 8, 0), GRAY, 1)  # ccr12
d.set_attrs(1, (10, 45), GRAY, 1)   # T1
d.set_attrs(2, (175, 45), GRAY, 1)   # K
d.set_attrs(3, (10, 87), GRAY, 1)   # T2
d.set_attrs(4, (175, 87), GRAY, 1)   # K
d.set_attrs(5, (430, 45), GRAY, 1)   # mbar
d.set_attrs(6, (430, 87), GRAY, 1)   # mbar
d.set_attrs(7, (40, 27), GREEN, 2)   # Wert1
d.set_attrs(8, (40, 69), GREEN, 2)   # Wert1
d.set_attrs(9, (255, 27), WHITE, 2)   # Press
d.set_attrs(10, (380, 20), WHITE, 2)  # PressExp
d.set_attrs(11, (360, 45), WHITE, 1)  # x10
d.set_attrs(12, (255, 69), RED, 2)    # --.--
d.set_attrs(13, (0, 112), ALARM, 1)  # Marquee

t = time.time()
t1 = 50
t2 = 10
p1 = 0.5

x = 226
yp = 10

d.set_font(0)
d.set_color(WHITE)
d.pos_text((205, 18), " 11")
d.pos_text((205, 101), "  9")
d.lines((225, 17), (225, 110))
d.set_color(GRAY)
d.lines((226, 61), (479, 61))

while 1:
    time.sleep(0.1)
    d.attr_text(0, "ccr12.kompass.frm2")
    d.attr_text(1, "T1")
    d.attr_text(2, "K")
    d.attr_text(3, "T2")
    d.attr_text(4, "K")
    #d.attr_text(5, "mbar")
    #d.attr_text(6, "mbar")

    rnd = random.random()
    t1 += rnd * 0.03
    if t1 > 230:
        t1 -= 50
    t2 += (rnd - 0.5) * 0.05

    if t1 >= 100:
        d.attr_text(7, "%6.2f" % t1)
    else:
        d.attr_text(7, "%6.3f" % t1)
    d.attr_text(8, "%6.3f" % t2)

    #d.attr_text(9, "%.3f" % (p1 + 0.1*(rnd - 0.5)))
    #d.attr_text(10, "-1")
    #d.attr_text(11, "x10")
    #d.attr_text(12, "-.---")

    if time.time() - t > 0.5:
        marq_off = (marq_off + 1) % marq_len
        t = time.time()

        x += 1
        if x >= 480:
            x = 226
            d.set_color(BLACK)
            d.rect((226, 16), (479, 109))
            d.set_color(GRAY)
            d.lines((226, 61), (478, 60))
        else:
            d.set_color(BLUE)
            y = min(max(int(61 - (t2-10)*50), 17), 110)
            d.lines((x-1, yp), (x, y))
            yp = y

    if marq_off + 60 <= marq_len:
        d.attr_text(13, MARQUEE[marq_off:marq_off+60])
    else:
        remain = marq_len - marq_off
        d.attr_text(13, MARQUEE[marq_off:] + MARQUEE[:60 - marq_len + marq_off])

    d.set_color(WHITE)
    d.lines((0, 16), (479, 16))
    d.lines((200, 16), (200, 111))
    d.lines((0, 111), (479, 111))
