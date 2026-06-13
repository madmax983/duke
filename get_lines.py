import xml.etree.ElementTree as ET
import sys

try:
    tree = ET.parse('cobertura.xml')
    root = tree.getroot()
    for package in root.findall('.//package'):
        for classes in package.findall('classes'):
            for cls in classes.findall('class'):
                if 'helpers.rs' in cls.get('filename') and 'telemetry' in cls.get('filename'):
                    lines = cls.find('lines')
                    uncovered = []
                    for line in lines.findall('line'):
                        if int(line.get('hits')) == 0:
                            uncovered.append(line.get('number'))
                    print("Uncovered lines in helpers.rs:", uncovered)
except Exception as e:
    print(e)
