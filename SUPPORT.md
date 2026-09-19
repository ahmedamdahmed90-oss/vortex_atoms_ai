# Support

## Getting Help

### Documentation

- [User Guide](docs/USER_GUIDE.md) - How to use Vortex Atoms AI
- [API Reference](docs/API_REFERENCE.md) - API documentation
- [Developer Guide](docs/DEVELOPER_GUIDE.md) - How to contribute
- [Architecture](docs/ARCHITECTURE.md) - System design

### Community

- [GitHub Discussions](https://github.com/mansour2024/vortex_atoms_ai/discussions) - Ask questions and share ideas
- [GitHub Issues](https://github.com/mansour2024/vortex_atoms_ai/issues) - Report bugs and request features

### Common Issues

#### Server won't start
- Check if port 8080 is already in use
- Verify Rust version (1.78+ required)
- Check the model path in vortex.json

#### Frontend won't connect
- Verify the backend is running: `curl http://localhost:8080/v1/health`
- Check the API URL in browser settings
- Ensure no firewall is blocking the connection

#### Out of memory
- Use a smaller model
- Reduce max_seq_len in config
- Close other applications

#### Slow responses
- Check CPU usage
- Use a smaller model
- Consider using a CUDA-enabled GPU

### Contact

- **GitHub Issues**: For bug reports and feature requests
- **GitHub Discussions**: For questions and general help
- **Email**: For security concerns (see SECURITY.md)

### Commercial Support

For commercial support, training, or consulting, please contact the project maintainers through GitHub.

---

## FAQ

**Q: Is this free to use?**
A: Yes! Vortex Atoms AI is open source and free to use under the MIT/Apache 2.0 license.

**Q: Can I use this commercially?**
A: Yes! The project is licensed under MIT/Apache 2.0, which allows commercial use.

**Q: How do I contribute?**
A: See our [Contributing Guide](CONTRIBUTING.md) for details.

**Q: Where can I report bugs?**
A: Use [GitHub Issues](https://github.com/mansour2024/vortex_atoms_ai/issues).

**Q: How do I request a feature?**
A: Use [GitHub Issues](https://github.com/mansour2024/vortex_atoms_ai/issues) with the "feature request" label.
