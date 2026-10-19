import base64, pathlib, shutil, socket, subprocess, sys, time
sys.argv = ['diag', 'generico', '--bmo']
sys.path.insert(0, str(pathlib.Path('scripts').resolve()))
import probar_arranque as t
img=t.BUILD/'diag-hyprland.img'
var=t.BUILD/'diag-hyprland.fd'
shutil.copyfile(t.IMG,img)
shutil.copyfile(t.QEMU_SHARE/'edk2-i386-vars.fd',var)
p=subprocess.Popen([str(t.QEMU),'-machine','pc','-accel','whpx','-cpu','host','-smp','2','-m','2048',
 '-vga','virtio','-drive',f'if=pflash,format=raw,readonly=on,file={t.QEMU_SHARE / "edk2-x86_64-code.fd"}',
 '-drive',f'if=pflash,format=raw,file={var}','-drive',f'file={img},format=raw,if=virtio',
 '-nic','none','-display','none','-no-reboot','-serial','tcp:127.0.0.1:15555,server=on,wait=on',
 '-monitor','tcp:127.0.0.1:15556,server=on,wait=off'])
out=''; failures=[]
try:
 for _ in range(50):
  try: c=socket.create_connection(('127.0.0.1',15555),timeout=1);break
  except OSError:time.sleep(.2)
 c.settimeout(.5)
 out+=t.read_until(c,'ultra:',120)
 for helper in ('ultra-cliente','ultra-hyprland'):
  data=base64.b64encode(pathlib.Path('src/rootfs/bin',helper).read_bytes()).decode()
  commands=[': > /tmp/helper.base64']
  commands += [f'printf %s {data[i:i+300]} >> /tmp/helper.base64' for i in range(0,len(data),300)]
  commands += [f'base64 -d /tmp/helper.base64 > /arch/usr/local/libexec/{helper}']
  for patch in commands:
   c.sendall((patch+'; echo PARCHE-$((2+2))\n').encode())
   part=t.read_until(c,'PARCHE-4',10)
   if 'ultra:' not in part.split('PARCHE-4')[-1]: part+=t.read_until(c,'ultra:',10)
   out+=part
  out+=t.run(c,[(f'ultra-entrar /usr/bin/bash -n /usr/local/libexec/{helper}; echo SINTAXIS-$?', 'SINTAXIS-0')],failures)
 out+=t.check_hyprland(c,failures)
 out+=t.run(c,[('cat /tmp/hyprland-session.log; echo FIN-DIAG','FIN-DIAG')],failures)
finally:
 p.kill();p.wait()
 pathlib.Path('build/diagnostico-hyprland.log').write_text(out,encoding='utf-8')
print(failures)
sys.exit(bool(failures))
