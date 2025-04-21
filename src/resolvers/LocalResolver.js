/**
 * @typedef {object} Resolver
 * @property {function(moddleElement: object, href: string): object} resolve
 */


export class LocalResolver {
  resolve(moddleElement, href) {
    if (!href.startsWith('#')) {
      return null;
    }

    const id = href.slice(1);
    const definitions = getDefinitions(moddleElement);

    return definitions.get('drgElement').find(e => e.id === id);
  }
}

function getDefinitions(moddleElement) {
  while (!moddleElement.$instanceOf('dmn:Definitions')) {
    moddleElement = moddleElement.$parent;

    if (!moddleElement) {
      throw new Error('No definitions found');
    }
  }

  return moddleElement;
}
