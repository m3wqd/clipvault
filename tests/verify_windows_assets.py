"""Check the shipped x64 PE contains the current Vite asset keys, without executing it."""
from html.parser import HTMLParser
from pathlib import Path
import struct
import sys

class References(HTMLParser):
    def __init__(self):
        super().__init__()
        self.paths = []
    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        value = attrs.get('src') if tag == 'script' else attrs.get('href') if tag == 'link' and attrs.get('rel') == 'stylesheet' else None
        if value:
            if value.startswith(('http:', 'https:', '//')):
                raise SystemExit('Frontend must not reference a remote script or stylesheet')
            self.paths.append(value.lstrip('/'))

if __name__ == '__main__':
    binary = Path(sys.argv[1])
    dist = Path(__file__).resolve().parents[1] / 'dist'
    data = binary.read_bytes()
    if data[:2] != b'MZ':
        raise SystemExit('Not a Windows executable')
    pe = struct.unpack_from('<I', data, 0x3c)[0]
    if data[pe:pe+4] != b'PE\0\0' or struct.unpack_from('<H', data, pe+4)[0] != 0x8664:
        raise SystemExit('Expected Windows x64 PE')
    refs = References()
    refs.feed((dist / 'index.html').read_text())
    if not refs.paths or not any(p.endswith('.js') for p in refs.paths):
        raise SystemExit('No frontend JavaScript reference found')
    for path in refs.paths:
        if not (dist / path).is_file() or path.encode() not in data:
            raise SystemExit('Current embedded asset missing: ' + path)
    print('Verified Windows x64 PE with current embedded JavaScript and CSS:', ', '.join(refs.paths))
