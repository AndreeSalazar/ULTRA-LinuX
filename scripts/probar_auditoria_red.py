#!/usr/bin/env python3
from auditar_red import audit

text = '''[INTERFAZ eth0: driver, enlace]
driver: r8169
Settings for eth0:
 Supported link modes: 10baseT/Full 100baseT/Full
                        1000baseT/Full
 Advertised link modes: 100baseT/Full 1000baseT/Full
 Link partner advertised link modes: 10baseT/Full
                                     100baseT/Full
 Link partner advertised pause frame use: No
 Speed: 100Mb/s
 Duplex: Full
 Auto-negotiation: on
rx-checksumming: on
receive-hashing: off [fixed]
[AUDIO: otra seccion]
driver: otro
'''
result = audit(text)
link, = result['interfaces']
assert link['max_anunciado_local_Mbps'] == 1000
assert link['max_anunciado_otro_extremo_Mbps'] == 100
assert link['campos']['driver'] == 'r8169'
assert link['offloads']['receive-hashing'] == 'off [fixed]'
assert link['campos']['rx_errors'] is None
assert result['pico_RX_intervalo_Mbps'] is None and result['softnet_descartes'] is None
assert not audit('sin datos')['interfaces']
missing = audit(text.replace(' Link partner advertised link modes: 10baseT/Full\n                                     100baseT/Full\n',''))['interfaces'][0]
assert missing['max_anunciado_otro_extremo_Mbps'] is None
print('VERDE: anuncios multilínea, limites de seccion, capacidades fijas y datos ausentes sin inventar ceros')
