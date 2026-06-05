# SubJudge Documentation

Welcome to the SubJudge API documentation.

## Documentation Files

### API Documentation

- **[API_OVERVIEW.md](./API_OVERVIEW.md)** - Complete REST API reference
  - All endpoints with request/response examples
  - Authentication and authorization (planned)
  - Data types and formats
  - Error handling patterns
  - Complete usage examples

- **[API_SYNC.md](./API_SYNC.md)** - Detailed sync API documentation
  - Upsert pattern explanation
  - Transaction safety for teams endpoint
  - Design decisions and architecture
  - Usage examples and best practices

### Rust API Documentation

Generate and view the Rust API documentation:

```bash
cargo doc --open
```

This will build HTML documentation from the Rust doc comments in the source code and open it in your browser.

## Quick Start

### View API Documentation

1. **REST API Reference**: Open [API_OVERVIEW.md](./API_OVERVIEW.md)
2. **Sync Operations**: See [API_SYNC.md](./API_SYNC.md)
3. **Rust Docs**: Run `cargo doc --open`

### API Modules

SubJudge provides three main API modules:

1. **Contest Management** (`/api/contests`)
   - Retrieve contest information
   - Modify contest timing (start time, thaw time)
   - Documented in: [src/api/contests.rs](../src/api/contests.rs)

2. **Access Control** (`/api/contests/{id}/access`)
   - Query client capabilities
   - Determine visible endpoints and properties
   - Role-based access (Public, Team, Admin)
   - Documented in: [src/api/access.rs](../src/api/access.rs)

3. **Data Synchronization** (`/api/sync`)
   - Bulk import from external sources
   - Idempotent upsert operations
   - Teams, Groups, Contests, Organizations
   - Documented in: [src/api/sync.rs](../src/api/sync.rs)

## Documentation Coverage

### ✅ Completed

- [x] Module-level documentation for all API modules
- [x] Function-level Rust doc comments with examples
- [x] REST API endpoint documentation
- [x] Request/response schemas
- [x] Error handling documentation
- [x] Usage examples and workflows
- [x] Architecture and design decisions
- [x] Data type specifications

### 🔄 In Progress

- [ ] Authentication/authorization implementation
- [ ] Database persistence for contest modifications
- [ ] Additional validation rules

## Contributing

When adding new API endpoints:

1. Add Rust doc comments (`///`) to all public functions
2. Include module-level documentation (`//!`) at the top of new files
3. Update API_OVERVIEW.md with the new endpoint details
4. Add examples and error scenarios
5. Run `cargo doc` to verify documentation builds

## Documentation Style

### Rust Doc Comments

```rust
/// Brief one-line summary.
///
/// Detailed description with multiple paragraphs if needed.
///
/// # Arguments
///
/// * `param1` - Description
///
/// # Returns
///
/// * `Ok(Type)` - Success case
/// * `Err(Error)` - Error case
///
/// # Examples
///
/// \`\`\`rust
/// // Code example
/// \`\`\`
pub async fn my_function(param1: Type) -> Result<Type, Error> {
    // implementation
}
```

### Markdown Documentation

- Use clear headings and hierarchy
- Include code examples for all operations
- Document error cases and edge conditions
- Provide complete request/response examples
- Explain design decisions and tradeoffs

## Building Documentation

### Rust Documentation

```bash
# Build documentation
cargo doc --no-deps

# Build and open in browser
cargo doc --open

# Include private items
cargo doc --document-private-items
```

### Checking for Issues

```bash
# Check for documentation warnings
cargo doc 2>&1 | grep warning
```

## Additional Resources

- [Rust Documentation Guidelines](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html)
- [REST API Best Practices](https://restfulapi.net/)
- [OpenAPI Specification](https://swagger.io/specification/) (for future API spec generation)
