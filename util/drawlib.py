#!/usr/bin/env python3

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

    def raw_text(self, sbytes):
        self.send(CMD_TEXT, sbytes)

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