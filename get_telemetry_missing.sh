cargo tarpaulin --out Xml
python3 -c "
import xml.etree.ElementTree as ET
root = ET.parse('cobertura.xml').getroot()
for package in root.findall('.//package'):
    for classes in package.findall('classes'):
        for cls in classes.findall('class'):
            if 'telemetry' in cls.attrib['filename']:
                print(cls.attrib['filename'])
                for line in cls.findall('.//line'):
                    if int(line.attrib['hits']) == 0:
                        print('  Line missing:', line.attrib['number'])
"
