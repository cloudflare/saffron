module.exports = {
    target: "node14.15",
    entry: {
        "cfron.test": "./tests/cfron.test.js"
    },
    output: {
        filename: "[name].js",
        path: __dirname + "/tests/bundle"
    },
    optimization: {
        minimize: false
    },
    experiments: {
        syncWebAssembly: true
    }
};