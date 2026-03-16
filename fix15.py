import re
content = open('crates/duke-interpreter/src/lib.rs').read()

content = content.replace('let _clinit_start', 'let clinit_start')
content = content.replace('clinit_start.elapsed().as_nanos() as u64', 'u64::try_from(clinit_start.elapsed().as_nanos()).unwrap_or(u64::MAX)')

content = content.replace('let _native_start', 'let native_start')
content = content.replace('native_start.elapsed().as_nanos() as u64', 'u64::try_from(native_start.elapsed().as_nanos()).unwrap_or(u64::MAX)')

content = content.replace('let _telem_exc_event_idx', 'let telem_exc_event_idx')
content = content.replace(' _telem_exc_event_idx,', ' telem_exc_event_idx,')

content = content.replace('let (_telem_name, _telem_pc, _telem_start)', 'let (telem_name, telem_pc, telem_start)')
content = content.replace('_telem_start.elapsed().as_nanos() as u64', 'u64::try_from(telem_start.elapsed().as_nanos()).unwrap_or(u64::MAX)')
content = content.replace(' _telem_name,', ' telem_name,')
content = content.replace(' _telem_pc,', ' telem_pc,')


open('crates/duke-interpreter/src/lib.rs', 'w').write(content)
