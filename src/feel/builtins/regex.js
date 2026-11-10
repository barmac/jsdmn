/**
 * Translation of XML Schema / XPath regular expressions (as used by the
 * FEEL `matches`, `replace` and `split` functions) to JavaScript.
 */

const BLOCKS = {
  'BasicLatin': '\\u0000-\\u007F',
  'Latin-1Supplement': '\\u0080-\\u00FF',
  'LatinExtended-A': '\\u0100-\\u017F',
  'LatinExtended-B': '\\u0180-\\u024F',
  'IPAExtensions': '\\u0250-\\u02AF',
  'SpacingModifierLetters': '\\u02B0-\\u02FF',
  'CombiningDiacriticalMarks': '\\u0300-\\u036F',
  'Greek': '\\u0370-\\u03FF',
  'GreekandCoptic': '\\u0370-\\u03FF',
  'Cyrillic': '\\u0400-\\u04FF',
  'Armenian': '\\u0530-\\u058F',
  'Hebrew': '\\u0590-\\u05FF',
  'Arabic': '\\u0600-\\u06FF',
  'Syriac': '\\u0700-\\u074F',
  'Thaana': '\\u0780-\\u07BF',
  'Devanagari': '\\u0900-\\u097F',
  'Bengali': '\\u0980-\\u09FF',
  'Thai': '\\u0E00-\\u0E7F',
  'Georgian': '\\u10A0-\\u10FF',
  'HangulJamo': '\\u1100-\\u11FF',
  'LatinExtendedAdditional': '\\u1E00-\\u1EFF',
  'GreekExtended': '\\u1F00-\\u1FFF',
  'GeneralPunctuation': '\\u2000-\\u206F',
  'SuperscriptsandSubscripts': '\\u2070-\\u209F',
  'CurrencySymbols': '\\u20A0-\\u20CF',
  'LetterlikeSymbols': '\\u2100-\\u214F',
  'NumberForms': '\\u2150-\\u218F',
  'Arrows': '\\u2190-\\u21FF',
  'MathematicalOperators': '\\u2200-\\u22FF',
  'BoxDrawing': '\\u2500-\\u257F',
  'GeometricShapes': '\\u25A0-\\u25FF',
  'MiscellaneousSymbols': '\\u2600-\\u26FF',
  'Dingbats': '\\u2700-\\u27BF',
  'CJKSymbolsandPunctuation': '\\u3000-\\u303F',
  'Hiragana': '\\u3040-\\u309F',
  'Katakana': '\\u30A0-\\u30FF',
  'CJKUnifiedIdeographs': '\\u4E00-\\u9FFF',
  'HangulSyllables': '\\uAC00-\\uD7AF',
  'PrivateUseArea': '\\uE000-\\uF8FF',
  'AlphabeticPresentationForms': '\\uFB00-\\uFB4F',
  'HalfwidthandFullwidthForms': '\\uFF00-\\uFFEF',
  'Specials': '\\uFFF0-\\uFFFF'
};

const NAME_START = 'A-Za-z_:\\u00C0-\\u00D6\\u00D8-\\u00F6\\u00F8-\\u02FF\\u0370-\\u037D\\u037F-\\u1FFF\\u200C-\\u200D\\u2070-\\u218F\\u2C00-\\u2FEF\\u3001-\\uD7FF\\uF900-\\uFDCF\\uFDF0-\\uFFFD';
const NAME_CHAR = `${ NAME_START }\\-.0-9\\u00B7\\u0300-\\u036F\\u203F-\\u2040`;

const SINGLE_CHAR_ESCAPES = 'nrt\\|.?*+(){}-[]^$';

export class RegexError extends Error {}

/**
 * @param {string} pattern
 * @param {string} [flags]
 *
 * @return {RegExp}
 */
export function createRegex(pattern, flags = '', extraFlags = '') {

  if (typeof flags !== 'string' || !/^[smixq]*$/.test(flags)) {
    throw new RegexError(`invalid regular expression flags <${ flags }>`);
  }

  let source;

  if (flags.includes('q')) {
    source = pattern.replace(/[.*+?^${}()|[\]\\/-]/g, '\\$&');
  } else {
    source = translate(flags.includes('x') ? stripWhitespace(pattern) : pattern);
  }

  const jsFlags = [
    'u',
    flags.includes('s') ? 's' : '',
    flags.includes('m') ? 'm' : '',
    flags.includes('i') ? 'i' : '',
    extraFlags
  ].join('');

  try {
    return new RegExp(source, jsFlags);
  } catch (error) {
    throw new RegexError(`invalid regular expression <${ pattern }>: ${ error.message }`);
  }
}

/**
 * Remove whitespace outside of character classes (`x` flag).
 */
function stripWhitespace(pattern) {

  let result = '';
  let depth = 0;
  let escaped = false;

  for (let idx = 0; idx < pattern.length; idx++) {
    const char = pattern[idx];

    if (depth === 0 && /[\t\n\r ]/.test(char)) {
      continue;
    }

    result += char;

    if (escaped) {
      escaped = false;
      continue;
    }

    if (char === '\\') {

      // outside of classes, whitespace between backslash and
      // escaped character is removed, too
      escaped = true;
      continue;
    }

    if (char === '[') {
      depth++;
    } else if (char === ']' && depth > 0) {
      depth--;
    }
  }

  return result;
}

function translate(pattern) {

  let result = '';
  let idx = 0;

  while (idx < pattern.length) {
    const char = pattern[idx];

    if (char === '\\') {
      const { source, length } = translateEscape(pattern, idx, false);

      result += source;
      idx += length;
      continue;
    }

    if (char === '[') {
      const { source, length } = translateClass(pattern, idx);

      result += source;
      idx += length;
      continue;
    }

    result += char;
    idx++;
  }

  return result;
}

/**
 * Translate an escape sequence starting at `idx`.
 */
function translateEscape(pattern, idx, inClass) {

  const next = pattern[idx + 1];

  if (next === undefined) {
    throw new RegexError('trailing backslash in regular expression');
  }

  if (next === 'p' || next === 'P') {
    const end = pattern.indexOf('}', idx);

    if (pattern[idx + 2] !== '{' || end === -1) {
      throw new RegexError('invalid category escape');
    }

    const name = pattern.slice(idx + 3, end);
    const negated = next === 'P';

    if (name.startsWith('Is')) {
      const block = BLOCKS[name.slice(2)];

      if (!block) {
        throw new RegexError(`unsupported block <${ name }>`);
      }

      return {
        source: inClass
          ? (negated ? unsupportedInClass(name) : block)
          : `[${ negated ? '^' : '' }${ block }]`,
        length: end - idx + 1
      };
    }

    return { source: `\\${ next }{${ name }}`, length: end - idx + 1 };
  }

  if ('sSdDwW'.includes(next)) {
    return { source: `\\${ next }`, length: 2 };
  }

  if (next === 'i' || next === 'c' || next === 'I' || next === 'C') {
    const chars = next.toLowerCase() === 'i' ? NAME_START : NAME_CHAR;
    const negated = next === 'I' || next === 'C';

    if (inClass) {
      return { source: negated ? unsupportedInClass(`\\${ next }`) : chars, length: 2 };
    }

    return { source: `[${ negated ? '^' : '' }${ chars }]`, length: 2 };
  }

  // back reference
  if (/[1-9]/.test(next) && !inClass) {
    return { source: `\\${ next }`, length: 2 };
  }

  if (SINGLE_CHAR_ESCAPES.includes(next)) {

    if (next === 'n' || next === 'r' || next === 't') {
      return { source: `\\${ next }`, length: 2 };
    }

    // `\-` is not a valid identity escape outside of classes in unicode mode
    if (next === '-' && !inClass) {
      return { source: '-', length: 2 };
    }

    return { source: `\\${ next }`, length: 2 };
  }

  throw new RegexError(`invalid escape <\\${ next }>`);
}

function unsupportedInClass(name) {
  throw new RegexError(`unsupported negated escape <${ name }> in character class`);
}

/**
 * Translate a character class starting at `idx`, supporting
 * class subtraction (`[a-z-[aeiou]]`).
 */
function translateClass(pattern, idx) {

  let position = idx + 1;

  let negated = false;

  if (pattern[position] === '^') {
    negated = true;
    position++;
  }

  let body = '';
  let subtraction = null;

  while (position < pattern.length) {
    const char = pattern[position];

    if (char === '\\') {
      const { source, length } = translateEscape(pattern, position, true);

      body += source;
      position += length;
      continue;
    }

    if (char === '-' && pattern[position + 1] === '[') {
      const { source, length } = translateClass(pattern, position + 1);

      subtraction = source;
      position += length + 1;

      if (pattern[position] !== ']') {
        throw new RegexError('character class subtraction must be the last part of a class');
      }

      continue;
    }

    if (char === '[') {
      throw new RegexError('nested character classes are not allowed');
    }

    if (char === ']') {
      position++;

      const cls = `[${ negated ? '^' : '' }${ body }]`;

      return {
        source: subtraction ? `(?:(?!${ subtraction })${ cls })` : cls,
        length: position - idx
      };
    }

    body += char;
    position++;
  }

  throw new RegexError('unterminated character class');
}

/**
 * Translate an XPath replacement string to a replacer function.
 *
 * @param {string} replacement
 *
 * @return {(match: string, ...groups: any[]) => string}
 */
export function createReplacer(replacement) {

  const parts = [];

  for (let idx = 0; idx < replacement.length; idx++) {
    const char = replacement[idx];

    if (char === '\\') {
      const next = replacement[idx + 1];

      if (next !== '\\' && next !== '$') {
        throw new RegexError(`invalid replacement string <${ replacement }>`);
      }

      parts.push(next);
      idx++;
      continue;
    }

    if (char === '$') {
      const match = /^\d+/.exec(replacement.slice(idx + 1));

      if (!match) {
        throw new RegexError(`invalid replacement string <${ replacement }>`);
      }

      parts.push({ group: Number(match[0]) });
      idx += match[0].length;
      continue;
    }

    parts.push(char);
  }

  return (match, ...args) => {

    // args: captured groups, offset, input
    const groups = args.slice(0, -2);

    return parts.map(part => {

      if (typeof part === 'string') {
        return part;
      }

      if (part.group === 0) {
        return match;
      }

      return groups[part.group - 1] ?? '';
    }).join('');
  };
}
