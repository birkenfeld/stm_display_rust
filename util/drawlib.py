#!/usr/bin/env python3

import sys
assert sys.version_info[0] == 3

CMD_MODE_GRAPHICS = 0x20
CMD_MODE_CONSOLE = 0x21

CMD_SET_POS = 0x30
CMD_SET_FONT = 0x31
CMD_SET_COLOR = 0x32
CMD_SET_CLIP = 0x33

CMD_CLEAR = 0x40
CMD_LINES = 0x41
CMD_RECT = 0x42
CMD_ICON = 0x43
CMD_TEXT = 0x44
CMD_COPYRECT = 0x45
CMD_PLOT = 0x46

CMD_TOUCH = 0x50
CMD_TOUCH_MODE = 0x51
CMD_TOUCH_CALIB = 0x52

CMD_SAVE_ATTRS = 0xa0
CMD_SAVE_ATTRS_MAX = 0xbf

CMD_SEL_ATTRS = 0xc0
CMD_SEL_ATTRS_MAX = 0xdf

CMD_BOOTMODE = 0xf0
CMD_RESET = 0xf1
CMD_SET_STARTUP = 0xf2
CMD_VERSION = 0xf3
CMD_RESET_APU = 0xf4

RESET_MAGIC = bytes([0xcb, 0xef, 0x20, 0x18])


class Display:
    def __init__(self, port):
        self.port = port
        self._record = None

    def _pos(self, xy):
        x, y = xy
        return bytes([y << 1 | (x > 0xff), x & 0xff])

    def send(self, cmd, argstr=b''):
        buf = bytearray(b'\x1b\x1b')
        if len(argstr) > 254:
            raise ValueError('command too long')
        buf.append(len(argstr) + 1)
        buf.append(cmd)
        buf.extend(argstr)
        if self._record is None:
            self.port.write(buf)
        else:
            self._record.append(buf)

    def _bootmode(self):
        self.send(CMD_BOOTMODE, RESET_MAGIC)

    def _reset(self):
        self.send(CMD_RESET, RESET_MAGIC)

    def get_version(self):
        self.send(CMD_VERSION)
        rsp = self.port.read(8)
        assert rsp[:4] == b'\x1b\x1b\x05%c' % CMD_VERSION
        return list(rsp[4:])

    def set_touch_mode(self, forward):
        self.send(CMD_TOUCH_MODE, b'\x01' if forward else b'\x00')

    def set_touch_calib(self, xd, xo, yd=1, yo=0):
        self.send(CMD_TOUCH_CALIB, bytes([xd, xo, yd, yo]))

    def touch_detect(self):
        rsp = self.port.read(6)
        if not rsp:
            return None
        assert rsp[:4] == b'\x1b\x1b\x03%c' % CMD_TOUCH
        return (rsp[4], rsp[5])

    def touch_detect_loop(self):
        while True:
            print('Touch:', *self.touch_detect())

    def record_startup(self):
        self._record = []

    def set_startup(self):
        cmds = b''.join(self._record)
        self._record = None
        self.send(CMD_SET_STARTUP, cmds)

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

    def set_clip(self, xy1, xy2):
        self.send(CMD_SET_CLIP, self._pos(xy1) + self._pos(xy2))

    def reset_clip(self):
        self.send(CMD_SET_CLIP)

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

    def raw_text(self, sbytes):
        self.send(CMD_TEXT, sbytes)

    def lines(self, *coords):
        buf = bytearray()
        for xy in coords:
            buf.extend(self._pos(xy))
        self.send(CMD_LINES, buf)

    def plot(self, x, y1, *ys):
        buf = bytearray(self._pos((x, y1)))
        for y in ys:
            buf.append(y)
        self.send(CMD_PLOT, buf)

    def rect(self, xy1, xy2):
        self.send(CMD_RECT, self._pos(xy1) + self._pos(xy2))

    def clear(self, color):
        self.send(CMD_CLEAR, bytes([color]))

    def copy_rect(self, xy1, xy2, dxy):
        self.send(CMD_COPYRECT, self._pos(xy1) + self._pos(xy2) +
                  self._pos(dxy))

    def icon(self, i):
        self.send(CMD_ICON, bytes([i]))
