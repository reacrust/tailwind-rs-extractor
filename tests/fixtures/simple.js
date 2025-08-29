// Simple test fixture with various string types
const className = "bg-blue-500 text-white p-4";
const singleClass = "hover:bg-blue-600";

// Template literals
const template = `flex items-center justify-between`;
const templateWithExpr = `text-${size}-xl font-bold`;

// Object with class names
const styles = {
  container: "max-w-7xl mx-auto",
  "header-title": "text-3xl font-semibold",
  button: 'inline-block px-6 py-3'
};

// Function returning classes
function getClasses(isDark) {
  if (isDark) {
    return "bg-gray-900 text-gray-100";
  }
  return 'bg-white text-gray-900';
}

// Array of classes
const classList = [
  "rounded-lg",
  'shadow-md',
  `border border-gray-200`
];