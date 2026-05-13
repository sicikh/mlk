export let conf = {
    brackets: [
        ['(', ')'],
        ['[', ']'],
        ['{', '}']
    ],
    colorizedBracketPairs: [],

    comments: {
        blockComment: ['/*', '*/']
    },

    autoClosingPairs: [
        { open: '(', close: ')', notIn: ['string', 'comment'] },
        { open: '"', close: '"', notIn: ['string', 'comment'] },
        { open: "'", close: "'", notIn: ['string', 'comment'] },
        { open: '[', close: ']', notIn: ['string', 'comment'] },
        { open: '{', close: '}', notIn: ['string', 'comment'] }
    ],

    folding: {
        markers: {
            start: new RegExp('^\\s*\\/\\*\\s*#region\\b\\s*(.*?)\\s*\\*\\/'),
            end: new RegExp('^\\s*\\/\\*\\s*#endregion\\b.*\\*\\/')
        }
    }
}

export let MlkMonarch  = {
    defaultToken: "invalid",
    ignoreCase: true,
    unicode: true,
    tokenPostfix: ".mlk",
    brackets: [
        { open: '(', close: ')', token: 'delimiter.parenthesis' },
        { open: '[', close: ']', token: 'delimiter.square' },
        { open: '{', close: '}', token: 'delimiter.curly' }
    ],
    keywords: [
        'let', 'fun', 'if', 'else', 'then',
    ],
    operators: [
        '=', '??', '||', '&&', '=', '!=', '<>', '<', '<=', '>', '>=',
        '!', '=>', '*', '-', '+', '/', '?', ':', '->', "|>"
    ],
    symbols: /[=><!?:&|+\-*\/]+/,
    tokenizer: {
        root: [
            [ /[()]/, '@brackets' ],
            [ /[\[\]]/, '@brackets' ],
            [ /[{}]/, '@brackets' ],

            [/[0-9][0-9_]*\.[0-9_]+/, 'number.float'],
            [/[0-9][0-9_]*/, 'number'],

            [/[;,.]/, 'delimiter'],

            // whitespace
            { include: '@whitespace' },

            [/@symbols/, {
                cases: {
                    '@operators': 'operators',
                    '@default': 'invalid'
                }
            }],

            // identifiers and keywords
            [/[a-zа-я_?][a-zа-я_\d]*/, {
                cases: {
                    '@keywords': { token: 'keyword.$0' },
                    '@default': 'identifier'
                }
            }],

            // strings
            [/"([^"\\]|\\.)*$/, 'string.invalid'],  // non-terminated string
            [/"/, { token: 'string.quote', next: '@string' }],
            [/'([^'\\]|\\.)*$/, 'string.invalid'],  // non-terminated string
            [/'/, { token: 'string.quote', next: '@string2' }],
        ],

        string: [
            [/[^\\"]+/, 'string'],
            [/"/, { token: 'string.quote', next: '@pop' }]
        ],
        string2: [
            [/[^\\']+/, 'string'],
            [/'/, { token: 'string.quote', next: '@pop' }]
        ],

        whitespace: [
            [/[ \t\r\n]+/, ''],
            [/\/\*/, 'comment', '@comment'],
            [/\/\/.*$/, 'comment'],
        ],

        comment: [
            [/[^\/*]+/, 'comment'],
            [/\/\*/, 'comment', '@push' ],
            [/\*\//, 'comment', '@pop'],
            [/[\/*]/, 'comment']
        ],
    }
};
