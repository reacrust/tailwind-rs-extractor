import React from 'react';

// Component with various conditional class patterns using real Tailwind classes
export default function ConditionalClasses({
  isActive = false,
  variant = 'primary',
  size = 'medium',
  hasError = false,
  items = []
}) {
  // Conditional without template literal - these static strings should be extracted
  const baseContainer = 'container mx-auto px-6 py-4';
  const containerState = isActive ? 'bg-green-100 border-green-500' : 'bg-gray-100 border-gray-300';
  const containerClass = baseContainer + ' ' + containerState;

  // Object-based variant mapping with real Tailwind
  const variantClasses = {
    primary: 'bg-indigo-600 hover:bg-indigo-700 text-white',
    secondary: 'bg-gray-600 hover:bg-gray-700 text-white',
    danger: 'bg-red-600 hover:bg-red-700 text-white'
  };

  // Size mapping with real Tailwind
  const sizeClasses = {
    small: 'text-xs py-1.5 px-3',
    medium: 'text-sm py-2 px-4',
    large: 'text-lg py-3 px-6'
  }[size] || 'text-sm py-2 px-4';

  // Array join pattern with real Tailwind
  const buttonClasses = [
    'rounded-md font-medium transition-all duration-200',
    variantClasses[variant],
    sizeClasses,
    hasError && 'ring-2 ring-red-400 ring-offset-2',
    isActive && 'shadow-lg transform scale-110'
  ].filter(Boolean).join(' ');

  // Inline ternary in JSX
  return (
    <div className={containerClass}>
      <button className={buttonClasses}>
        Click Me
      </button>

      {/* Inline conditional with ternary - avoids template literal */}
      <div className={hasError ? 'mt-6 p-4 rounded-lg bg-red-50 border border-red-400' : 'mt-6 p-4 rounded-lg'}>
        {hasError ? 'Error occurred' : 'All good'}
      </div>

      {/* Dynamic classes in map - using static classes that extractor can find */}
      <ul className="divide-y divide-gray-200 my-8">
        {items.map((item, idx) => (
          <li
            key={idx}
            className="py-3 px-4 hover:bg-gray-100"
          >
            {item}
          </li>
        ))}
      </ul>

      {/* Complex nested conditional */}
      <div
        className={
          isActive
            ? (hasError ? 'text-red-700 font-bold uppercase' : 'text-green-700 font-semibold italic')
            : 'text-gray-600 font-normal'
        }
      >
        Status indicator
      </div>

      {/* Spread with className override */}
      <span className="inline-flex px-3 py-1 bg-blue-100 text-blue-800 rounded-full" {...(hasError && { className: 'inline-flex px-3 py-1 bg-red-100 text-red-800 rounded-full' })}>
        Badge
      </span>
    </div>
  );
}