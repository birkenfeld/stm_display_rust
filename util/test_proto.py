#!/usr/bin/env python3
import sys
import time
import serial
import random

CMD_MODE_GRAPHICS = 0x20
CMD_MODE_CONSOLE = 0x21

CMD_SET_POS = 0x30
CMD_SET_FONT = 0x31
CMD_SET_COLOR = 0x32

CMD_SAVE_ATTRS = 0x40
CMD_SAVE_ATTRS_MAX = 0x4f

CMD_SEL_ATTRS = 0x50
CMD_SEL_ATTRS_MAX = 0x5f

CMD_TEXT = 0x60
CMD_LINES = 0x61
CMD_RECT = 0x62
CMD_ICON = 0x63
CMD_CLEAR = 0x64


class Display:
    def __init__(self, port):
        self.port = port

    def _pos(self, xy):
        x, y = xy
        return bytes([y << 1 | (x > 0xff), x & 0xff])

    def send(self, cmd, argstr=b''):
        buf = bytearray(b'\x1b\x1b')
        buf.append(len(argstr) + 1)
        buf.append(cmd)
        buf.extend(argstr)
        self.port.write(buf)

    def switch_console(self):
        self.send(CMD_MODE_CONSOLE)

    def switch_graphics(self):
        self.send(CMD_MODE_GRAPHICS)

    def set_pos(self, xy):
        self.send(CMD_SET_POS, self._pos(xy))

    def set_font(self, i):
        self.send(CMD_SET_FONT, bytes([i]))

    def set_color(self, colors):
        self.send(CMD_SET_COLOR, bytes(colors))

    def save_attrs(self, i):
        self.send(CMD_SAVE_ATTRS + i)

    def select_attrs(self, i):
        self.send(CMD_SEL_ATTRS + i)

    def pos_text(self, pos, string):
        self.set_pos(pos)
        self.text(string)

    def attr_text(self, i, string):
        self.select_attrs(i)
        self.text(string)

    def set_attrs(self, i, pos, colors, font):
        self.set_pos(pos)
        self.set_color(colors)
        self.set_font(font)
        self.save_attrs(i)

    def text(self, string):
        self.send(CMD_TEXT, string.encode('cp437'))

    def lines(self, *coords):
        buf = bytearray()
        for xy in coords:
            buf.extend(self._pos(xy))
        self.send(CMD_LINES, buf)

    def rect(self, xy1, xy2):
        self.send(CMD_RECT, self._pos(xy1) + self._pos(xy2))

    def clear(self, color):
        self.send(CMD_CLEAR, bytes([color]))

    def icon(self, i):
        self.send(CMD_ICON, bytes([i]))


s = serial.Serial(sys.argv[1], baudrate=115200)

d = Display(s)

GRAY = [0, 239, 240, 245]
WHITE = [0, 239, 247, 255]
RED = [0, 52, 124, 196]
GREEN = [0, 28, 34, 46]
ALARM = [1, 197, 223, 255]

MARQUEE = ("compressor off, cooling water temperature alarm, "
           "cold head has spontaneously combusted --- ")
marq_len = len(MARQUEE)
marq_off = 0

if sys.argv[2] == '1':
    d.clear(255)
    d.set_pos((120, 7))
    d.set_color([60, 104, 188, 255])
    d.icon(0)
    d.set_color([255, 248, 235, 0])
    d.set_font(1)
    d.pos_text((120, 103), "Booting...".center(30))
    d.switch_graphics()

    time.sleep(3)
    d.pos_text((120, 103), "Getting network address...".center(30))
    time.sleep(3)
    d.pos_text((120, 103), "Starting SeCOP servers...".center(30))
    time.sleep(3)
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

    while 1:
        d.attr_text(0, "ccr12.kompass.frm2")
        d.attr_text(1, "T1")
        d.attr_text(2, "K")
        d.attr_text(3, "T2")
        d.attr_text(4, "K")
        d.attr_text(5, "mbar")
        d.attr_text(6, "mbar")

        rnd = random.random()
        t1 += rnd * 0.03
        if t1 > 230:
            t1 -= 50
        t2 += (rnd - 0.5) * 0.01

        if t1 >= 100:
            d.attr_text(7, "%6.2f" % t1)
        else:
            d.attr_text(7, "%6.3f" % t1)
        d.attr_text(8, "%6.3f" % t2)

        d.attr_text(9, "%.3f" % (p1 + 0.1*(rnd - 0.5)))
        d.attr_text(10, "-1")
        d.attr_text(11, "x10")
        d.attr_text(12, "-.---")

        if time.time() - t > 0.5:
            marq_off = (marq_off + 1) % marq_len
            t = time.time()
        if marq_off + 60 <= marq_len:
            d.attr_text(13, MARQUEE[marq_off:marq_off+60])
        else:
            remain = marq_len - marq_off
            d.attr_text(13, MARQUEE[marq_off:] + MARQUEE[:60 - marq_len + marq_off])

        d.set_color(WHITE)
        d.lines((0, 16), (479, 16))
        d.lines((220, 16), (220, 111))
        d.lines((0, 111), (479, 111))

else:
    d.switch_console()
