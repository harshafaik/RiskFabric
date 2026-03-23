// Convert mermaid code blocks to <pre class="mermaid"> tags
document.addEventListener('DOMContentLoaded', () => {
  document.querySelectorAll('code.language-mermaid').forEach((block) => {
    const pre = document.createElement('pre');
    pre.className = 'mermaid';
    pre.textContent = block.textContent;
    const parentPre = block.parentElement;
    if (parentPre && parentPre.tagName === 'PRE') {
      parentPre.replaceWith(pre);
    }
  });

  // Load Mermaid via script tag
  const script = document.createElement('script');
  script.src = 'https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.min.js';
  script.onload = () => {
    const getMermaidTheme = () => {
      const theme = document.documentElement.className;
      if (theme.includes('ayu') || theme.includes('coal') || theme.includes('navy')) {
        return 'dark';
      }
      return 'default';
    };

    mermaid.initialize({
      startOnLoad: true,
      theme: getMermaidTheme(),
      securityLevel: 'loose',
      fontFamily: 'inherit',
      themeVariables: {
        primaryColor: '#ff8c00', // Rust-like primary
        lineColor: '#808080',
      }
    });

    // Handle theme changes
    const observer = new MutationObserver(() => {
      const newTheme = getMermaidTheme();
      // We need to reload to apply theme properly in some Mermaid versions
      // but reinitialize often works for basic color shifts
      mermaid.initialize({ ...mermaid.config, theme: newTheme });
      mermaid.init();
    });
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] });
  };
  document.head.appendChild(script);
});
