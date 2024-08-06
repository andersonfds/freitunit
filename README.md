# freitool: JUnit XML output from Flutter Pipe

This project outputs JUnit XML from Flutter test output.
I plan on adding more customization options and features in the future.
For now it will only output the [JUnit XML](https://llg.cubic.org/docs/junit/). One for each test suite.

[!["Buy Me A Coffee"](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://www.buymeacoffee.com/andersonfds)

Usage:

```sh
flutter test --machine | freitunit
```

Installation:

```sh
brew tap andersonfds/freitunit
brew install freitunit
```

Features:
- Outputs to the console only the errors in a more readable format
- Saves each testsuite under its own `reports.xml` file
