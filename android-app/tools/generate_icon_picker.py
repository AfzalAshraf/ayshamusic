from pathlib import Path
from PIL import Image,ImageDraw
r=Path('android-app/app/src/main/res'); sources={
'classic':'android-app/artwork/aysha-launcher.png','rose':'android-app/artwork/romantic-launcher.png',
'velvet':'design/ay-sha-mark/aysha-velvet-rose.png','cute':'design/ay-sha-mark/aysha-cute-rose.png',
'red':'design/ay-sha-mark/aysha-racing-red.png','ivory':'design/ay-sha-mark/aysha-red-on-ivory.png',
'butterflies':'design/ay-sha-mark/aysha-red-butterflies.png','embossed':'design/ay-sha-mark/aysha-embossed-transparent.png'}
for name,path in sources.items():
 src=Image.open(path).convert('RGBA'); base=Image.new('RGBA',(512,512),'#fff8f0' if name in ['ivory','butterflies','embossed'] else '#210817')
 # Optical fit inside the adaptive safe circle; embossed cutout needs padding.
 if name=='embossed':
  src.thumbnail((264,300),Image.Resampling.LANCZOS);base.alpha_composite(src,((512-src.width)//2,(512-src.height)//2))
 elif name=='classic':
  src=src.resize((276,276),Image.Resampling.LANCZOS);base.alpha_composite(src,(118,118))
 else:
  src=src.resize((512,512),Image.Resampling.LANCZOS);base.alpha_composite(src,(0,0))
 base.convert('RGB').save(r/f'drawable-nodpi/icon_{name}_art.png',optimize=True)
 crop=base.crop((64,64,448,448))
 for density,size in [('mdpi',48),('hdpi',72),('xhdpi',96),('xxhdpi',144),('xxxhdpi',192)]:
  image=crop.resize((size,size),Image.Resampling.LANCZOS);mask=Image.new('L',(size*4,size*4));ImageDraw.Draw(mask).rounded_rectangle((0,0,size*4-1,size*4-1),radius=size*.23*4,fill=255);image.putalpha(mask.resize((size,size),Image.Resampling.LANCZOS));image.save(r/f'mipmap-{density}/ic_{name}.png',optimize=True)
 preview=crop.resize((96,96),Image.Resampling.LANCZOS);preview.save(r/f'drawable-nodpi/icon_{name}_preview.png',optimize=True)
 (r/f'mipmap-anydpi-v26/ic_{name}.xml').write_text(f'<?xml version="1.0" encoding="utf-8"?>\n<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android"><background android:drawable="@color/launcher_background"/><foreground android:drawable="@drawable/icon_{name}_art"/></adaptive-icon>\n')
