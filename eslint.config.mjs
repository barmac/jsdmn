import bpmnIoPlugin from 'eslint-plugin-bpmn-io';

const files = {
  src: [
    'src/**/*.js'
  ],
  test: [
    'test/**/*.js'
  ]
};

export default [

  // build + test
  ...bpmnIoPlugin.configs.node.map(config => {

    return {
      ...config,
      ignores: files.src
    };
  }),

  // src
  ...bpmnIoPlugin.configs.recommended.map(config => {

    return {
      ...config,
      files: files.src
    };
  }),

  // test
  ...bpmnIoPlugin.configs.mocha.map(config => {

    return {
      ...config,
      files: files.test
    };
  }),
  {
    languageOptions: {
      globals: {
        sinon: true
      }
    },
    files: files.test
  }
];