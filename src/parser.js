import dmnModdle from 'dmn-moddle';

export function parseDmn(dmn) {
  return dmnModdle().fromXML(dmn);
}
