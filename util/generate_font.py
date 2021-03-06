#!/usr/bin/env python3

import os
import sys
import subprocess
import cairo

# Note: NUL replaced by FFFD since it cannot be drawn with pycairo
CP437 = [
    0xFFFD, 0x263A, 0x263B, 0x2665, 0x2666, 0x2663, 0x2660, 0x2022,  # 0x
    0x25D8, 0x25CB, 0x25D9, 0x2642, 0x2640, 0x266A, 0x266B, 0x263C,
    0x25BA, 0x25C4, 0x2195, 0x203C, 0x00B6, 0x00A7, 0x25AC, 0x21A8,  # 1x
    0x2191, 0x2193, 0x2192, 0x2190, 0x221F, 0x2194, 0x25B2, 0x25BC,
    0x0020, 0x0021, 0x0022, 0x0023, 0x0024, 0x0025, 0x0026, 0x0027,  # 2x
    0x0028, 0x0029, 0x002a, 0x002b, 0x002c, 0x002d, 0x002e, 0x002f,
    0x0030, 0x0031, 0x0032, 0x0033, 0x0034, 0x0035, 0x0036, 0x0037,  # 3x
    0x0038, 0x0039, 0x003a, 0x003b, 0x003c, 0x003d, 0x003e, 0x003f,
    0x0040, 0x0041, 0x0042, 0x0043, 0x0044, 0x0045, 0x0046, 0x0047,  # 4x
    0x0048, 0x0049, 0x004a, 0x004b, 0x004c, 0x004d, 0x004e, 0x004f,
    0x0050, 0x0051, 0x0052, 0x0053, 0x0054, 0x0055, 0x0056, 0x0057,  # 5x
    0x0058, 0x0059, 0x005a, 0x005b, 0x005c, 0x005d, 0x005e, 0x005f,
    0x0060, 0x0061, 0x0062, 0x0063, 0x0064, 0x0065, 0x0066, 0x0067,  # 6x
    0x0068, 0x0069, 0x006a, 0x006b, 0x006c, 0x006d, 0x006e, 0x006f,
    0x0070, 0x0071, 0x0072, 0x0073, 0x0074, 0x0075, 0x0076, 0x0077,  # 7x
    0x0078, 0x0079, 0x007a, 0x007b, 0x007c, 0x007d, 0x007e, 0x2302,
    0x00c7, 0x00fc, 0x00e9, 0x00e2, 0x00e4, 0x00e0, 0x00e5, 0x00e7,  # 8x
    0x00ea, 0x00eb, 0x00e8, 0x00ef, 0x00ee, 0x00ec, 0x00c4, 0x00c5,
    0x00c9, 0x00e6, 0x00c6, 0x00f4, 0x00f6, 0x00f2, 0x00fb, 0x00f9,  # 9x
    0x00ff, 0x00d6, 0x00dc, 0x00a2, 0x00a3, 0x00a5, 0x20a7, 0x0192,
    0x00e1, 0x00ed, 0x00f3, 0x00fa, 0x00f1, 0x00d1, 0x00aa, 0x00ba,  # Ax
    0x00bf, 0x2310, 0x00ac, 0x00bd, 0x00bc, 0x00a1, 0x00ab, 0x00bb,
    0x2591, 0x2592, 0x2593, 0x2502, 0x2524, 0x2561, 0x2562, 0x2556,  # Bx
    0x2555, 0x2563, 0x2551, 0x2557, 0x255d, 0x255c, 0x255b, 0x2510,
    0x2514, 0x2534, 0x252c, 0x251c, 0x2500, 0x253c, 0x255e, 0x255f,  # Cx
    0x255a, 0x2554, 0x2569, 0x2566, 0x2560, 0x2550, 0x256c, 0x2567,
    0x2568, 0x2564, 0x2565, 0x2559, 0x2558, 0x2552, 0x2553, 0x256b,  # Dx
    0x256a, 0x2518, 0x250c, 0x2588, 0x2584, 0x258c, 0x2590, 0x2580,
    0x03b1, 0x00df, 0x0393, 0x03c0, 0x03a3, 0x03c3, 0x00b5, 0x03c4,  # Ex
    0x03a6, 0x0398, 0x03a9, 0x03b4, 0x221e, 0x03c6, 0x03b5, 0x2229,
    0x2261, 0x00b1, 0x2265, 0x2264, 0x2320, 0x2321, 0x00f7, 0x2248,  # Fx
    0x00b0, 0x2219, 0x00b7, 0x221a, 0x207f, 0x00b2, 0x25a0, 0x00a0,
]

# Selection of material icons (index relative to the private use area 0xE000)
MATERIAL = [
    0x000, 0x001, 0x002, 0x01D, 0x01F, 0x020, 0x028, 0x029,  # 0x
    0x02B, 0x02E, 0x02F, 0x031, 0x033, 0x034, 0x035, 0x037,
    0x038, 0x03B, 0x03C, 0x040, 0x042, 0x043, 0x044, 0x045,  # 1x
    0x047, 0x048, 0x04A, 0x04D, 0x04E, 0x04F, 0x050, 0x055,
    0x05C, 0x061, 0x065, 0x0B2, 0x0B4, 0x0B5, 0x0B7, 0x0BE,  # 2x
    0x0C3, 0x0C6, 0x0C8, 0x0CB, 0x0CE, 0x0D8, 0x0DA, 0x0DF,
    0x0E4, 0x0E5, 0x145, 0x146, 0x147, 0x148, 0x14A, 0x14B,  # 3x
    0x14C, 0x14D, 0x14E, 0x14F, 0x150, 0x152, 0x153, 0x154,
    0x157, 0x158, 0x15A, 0x15B, 0x15C, 0x15E, 0x15F, 0x160,  # 4x
    0x166, 0x190, 0x192, 0x226, 0x255, 0x258, 0x259, 0x25A,
    0x2C4, 0x2C6, 0x2C7, 0x2C8, 0x2CC, 0x30C, 0x313, 0x314,  # 5x
    0x315, 0x316, 0x317, 0x318, 0x31B, 0x31C, 0x332, 0x335,
    0x3AD, 0x3C2, 0x3E7, 0x417, 0x420, 0x429, 0x55C, 0x56B,  # 6x
    0x63E, 0x645, 0x7F3, 0x80E, 0x869, 0x876, 0x87D, 0x88A,
    0x88B, 0x898, 0x89A, 0x8B8, 0x8BE, 0x92B, 0xB3B, 0xB3E,  # 7x
]

try:
    _, family, style, size, ht, wd, xof, yof, cs, out = sys.argv
    size = int(size)
    ht = int(ht)
    wd = int(wd)
    xof = int(xof)
    yof = int(yof)
except Exception:
    print('usage: generate_font.py fontname fontstyle fontsize charheight '
          'charwidth xoffset yoffset charset outfilename')
    print()
    print('charset is either "ascii", "cp437" or a string of ASCII chars')
    sys.exit(1)
print(f'generating for {family} size {size}')

# Determine the glyphs to be rendered.

if cs == 'ascii':
    cs = ['\ufffd'] + [chr(i) for i in range(1, 128)] + ['\ufffd'] * 128
elif cs == 'cp437':
    cs = [chr(i) for i in CP437]
elif cs == 'material':
    cs = [chr(0xE000 + i) for i in MATERIAL]
else:
    new = ['\ufffd'] * 256
    for ch in cs:
        try:
            idx = CP437.index(ord(ch))
        except Exception:
            print('error: char {!r} is not in CP437!'.format(ch))
            sys.exit(1)
        new[idx] = ch
    cs = new

# Minimal checking if the font is actually present.

try:
    subprocess.check_call(['fc-list', '-q', family])
except subprocess.CalledProcessError:
    print('error: the requested font {!r} does not seem to be '
          'installed'.format(family))
    sys.exit(1)

# Generate the 2-bit per pixel bitmap for each glyph.

chrbuf = bytearray(wd * ht * 4)
padding = bytes([0] * ((-wd * ht) % 4))

weight = cairo.FontWeight.BOLD if 'Bold' in style else cairo.FontWeight.NORMAL
slant = cairo.FontSlant.ITALIC if 'Italic' in style else cairo.FontSlant.NORMAL

glyphs = {}

for (i, ch) in enumerate(cs):
    surface = cairo.ImageSurface.create_for_data(chrbuf, cairo.Format.RGB24,
                                                 wd, ht)
    ctx = cairo.Context(surface)
    ctx.select_font_face(family, slant, weight)
    ctx.set_font_size(size)

    extent = ctx.text_extents(ch)
    ch_width = extent.x_advance
    xof_ch = xof
    if abs(ch_width - wd) > 0.001:
        if ch.isprintable() and ch != '\ufffd':
            if extent.width > wd:
                print('warning: char {} is too wide (by {}) and will be '
                      'clipped'.format(ch, ch_width - wd))
            else:
                print('note: char {} is too {} (by {}), adjusting'.format(
                    ch, 'wide' if ch_width > wd else 'thin',
                    abs(ch_width - wd)))
        xof_ch += (wd - ch_width) / 2.0
    if ch.isprintable() and ch != '\ufffd':
        if ht + extent.y_bearing + yof < -0.001:
            print('warning: char {} is too high (by {}) and will be '
                  'clipped'.format(ch, abs(ht + extent.y_bearing + yof)))
        if extent.y_bearing + extent.height + yof > 0.001:
            print('warning: char {} is too low (by {}) and will be '
                  'clipped'.format(ch, extent.y_bearing + extent.height + yof))

    ctx.set_source_rgb(0.0, 0.0, 0.0)
    ctx.paint()
    ctx.set_source_rgb(0.0, 0.0, 1.0)
    ctx.move_to(xof_ch, ht + yof)
    ctx.show_text(ch)
    surface.finish()

    # Get rid of superfluous color/alpha channels and add padding to make the
    # result length a multiple of 4 (to be completely encoded).
    buf = chrbuf[::4] + padding
    # Change colors to 2-bit per pixel.
    # XXX: Change the distribution of 0-255 to 0,85,170,255?
    bytebuf = bytearray(
        (buf[i+3] // 64) << 6 |
        (buf[i+2] // 64) << 4 |
        (buf[i+1] // 64) << 2 |
        (buf[i]   // 64)
        for i in range(0, len(buf), 4))
    # Determine amount of leading and trailing empty space to more efficiently
    # encode the different glyphs.
    leading = len(bytebuf) - len(bytebuf.lstrip(b'\x00'))
    trailing = len(bytebuf) - len(bytebuf.rstrip(b'\x00'))
    glyphs[i, leading, trailing] = bytebuf

# Try to heuristically minimize the size of the data stream:
# - If a character's bitmap already occurs somewhere, use that index.
# - Otherwise, try to put characters with a matching amount of leading
#   and trailing empty space next to each other.

last_trailing = 0

data = bytearray()
indices = [0] * 256

while glyphs:
    matching = sorted(glyphs, key=lambda info: abs(info[1] - last_trailing))[0]
    glyphdata = glyphs.pop(matching)
    glyphindex = matching[0]

    # check for complete match within data so far
    for i in range(len(data)):
        if data[i:i+len(glyphdata)] == glyphdata:
            indices[glyphindex] = i
            break
    else:
        # check for partial match at the end
        for i in range(len(glyphdata), 0, -1):
            if data[-i:] == glyphdata[:i]:
                indices[glyphindex] = len(data) - i
                data.extend(glyphdata[i:])
                break
        else:
            # no match found.
            indices[glyphindex] = len(data)
            data.extend(glyphdata)
    last_trailing = matching[2]


datout = out.replace('.rs', '') + '.dat'
open(datout, 'wb').write(data)
open(out, 'w').write('''\
Font {{
    data:  include_bytes!("{}"),
    chars: {},
    charw: {},
    charh: {},
}}
'''.format(
    os.path.basename(datout),
    str(indices),
    wd,
    ht,
))
