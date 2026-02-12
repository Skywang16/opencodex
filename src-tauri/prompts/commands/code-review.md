Perform a comprehensive code review of the specified code or files.

{{input}}

## Code Review Guidelines

Analyze the code across these dimensions:

### 1. Code Quality
- Identify code smells and anti-patterns
- Check adherence to language-specific best practices
- Evaluate naming conventions and code organization
- Assess complexity and suggest simplifications

### 2. Security
- Identify potential security vulnerabilities (OWASP Top 10)
- Check for injection flaws, XSS, authentication issues
- Review data validation and sanitization
- Assess secrets management and sensitive data handling

### 3. Performance
- Identify performance bottlenecks
- Check for inefficient algorithms or data structures
- Review database queries and API calls
- Suggest optimization opportunities

### 4. Maintainability
- Assess code readability and clarity
- Evaluate documentation quality
- Check for proper error handling
- Review code duplication and modularity

### 5. Testing
- Evaluate test coverage and quality
- Identify missing test cases
- Review test structure and assertions
- Suggest improvements to testing strategy

## Output Format

Provide specific, actionable feedback with:
- File paths and line numbers for each issue
- Severity level (Critical/High/Medium/Low)
- Clear explanation of the problem
- Concrete suggestions for improvement
- Code examples where helpful

Focus on issues that matter most for code quality, security, and maintainability.

