![Mainstage](./media/mainstage_logo_text.svg)

# Mainstage

[![License](https://img.shields.io/badge/License-blue.svg)](LICENSE.md)  [![GitHub issues](https://img.shields.io/github/issues/ColtMcG1/mainstage)](https://github.com/ColtMcG1/mainstage/issues)  [![GitHub forks](https://img.shields.io/github/forks/ColtMcG1/mainstage)](https://github.com/ColtMcG1/mainstage/forks)  [![GitHub stars](https://img.shields.io/github/stars/ColtMcG1/mainstage)](https://github.com/ColtMcG1/mainstage/stars)

Mainstage is a scripting language designed for cross-platform orchestration and automation tasks. It aims to provide a simple and intuitive syntax for defining workflows. Its design focuses on readability and ease of use, making it accessible for both beginners and experienced developers. Its designed to be extensible, allowing users to create custom modules and plugins to enhance its functionality. It is not designed for a specific domain, but rather to be a general-purpose tool that can be adapted to a wide range of use cases and expanded as needed via its modular architecture.

## Features

- Cross-platform compatibility
- Intuitive syntax for workflow definition
- Extensible architecture with custom modules and plugins

## Installation

To install Mainstage, follow these steps:

- If you are running the local installer, download the installer from the [official website](https://github.com/ColtMcG1/mainstage/releases) and follow the on-screen instructions.
- If you are using a package manager, you can install Mainstage using the following command:

    ```bash
    # Using Homebrew (macOS/Linux)
    brew install mainstage
    # Using Chocolatey (Windows)
    choco install mainstage
    ```

- If you prefer manual installation, follow these steps:

    1. Download the latest release from the [official repository](https://github.com/ColtMcG1/mainstage/releases).
    2. Extract the downloaded archive to your desired location.
    3. Add the Mainstage binary to your system's PATH for easy access.
    4. Verify the installation by running `mainstage --version` in your terminal.

## Getting Started

To get started with Mainstage, create a new script file with the `.ms` extension. Here is a simple example of a Mainstage script:

```mainstage
workspace hello_world {
    say("Hello, World!");
}
```

To run the script, use the following command in your terminal:

```bash
mainstage your_script.ms
```

This will execute the script and print "Hello, World!" to the console.

See the `examples/` directory for more sample scripts and use cases.

## Documentation

For detailed documentation on Mainstage, including syntax, built-in functions, and examples, please visit the [official documentation site](https://github.com/ColtMcG1/mainstage/wiki).

See the `docs/` directory for local documentation files.

- GRAMMAR: `docs/GRAMMAR.md`
- MSBC SPEC: `docs/MSBC_SPEC.md`

### Testing

To run the test suite for Mainstage, navigate to the `core/` directory and execute the following command:

```bash
cargo test
```

Or to run tests by category, use the provided script:

```powershell
.\scripts\run_core_tests_by_category.ps1 -Category [CategoryName]
```

- Replace `[CategoryName]` with the desired test category (e.g., `lowering`, `opt`, `ir`, etc.).

## Contributing

Contributions to Mainstage are welcome! If you would like to contribute, please follow these steps:

1. Fork the repository on GitHub.
2. Create a new branch for your feature or bug fix.
3. Make your changes and commit them with clear messages.
4. Push your changes to your forked repository.
5. Submit a pull request to the main repository.

Please ensure that your code adheres to the project's coding standards and includes appropriate tests, documentation, and comments.

## License

See the [LICENSE](LICENSE.md) file for license rights and limitations.

## Contact

For questions or support, please open an issue on the [GitHub repository](https://github.com/ColtMcG1/mainstage/issues).

## Acknowledgments

We would like to thank all contributors and users who have supported the development of Mainstage. Your feedback and contributions are invaluable to the growth of this project.

---

Thank you for using Mainstage!
