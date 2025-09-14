import React from 'react';

// Component with conditional Tailwind classes using real patterns
export default function ConditionalTailwind({
  isActive = false,
  variant = 'primary',
  size = 'medium',
  hasError = false,
  items = []
}) {
  // Template literal conditional with real Tailwind classes
  const containerClass = `mx-auto px-6 ${isActive ? 'bg-emerald-600' : 'bg-slate-700'}`;

  // Object-based variant mapping with real Tailwind
  const variantClasses = {
    primary: 'bg-violet-600 hover:bg-violet-700',
    secondary: 'bg-cyan-600 hover:bg-cyan-700',
    danger: 'bg-rose-600 hover:bg-rose-700'
  };

  // Size mapping with real Tailwind
  const sizeClasses = {
    small: 'text-xs py-1 px-2',
    medium: 'text-base py-2 px-4',
    large: 'text-xl py-3 px-6'
  }[size] || 'text-base py-2 px-4';

  // Array join pattern with real Tailwind
  const buttonClasses = [
    'rounded-lg font-semibold transition-colors',
    variantClasses[variant],
    sizeClasses,
    hasError && 'ring-4 ring-red-500',
    isActive && 'shadow-2xl scale-105'
  ].filter(Boolean).join(' ');

  // Inline ternary in JSX
  return (
    <div className={containerClass}>
      <button className={buttonClasses}>
        Click Me
      </button>

      {/* Inline conditional with logical AND */}
      <div className={`mt-4 p-3 rounded ${hasError && 'border-2 border-red-600 bg-red-50'}`}>
        {hasError ? 'Error occurred' : 'All good'}
      </div>

      {/* Dynamic classes in map */}
      <ul className="space-y-2 my-6">
        {items.map((item, idx) => (
          <li
            key={idx}
            className={`px-3 py-1 ${idx === 0 ? 'font-bold text-lg' : ''} ${idx % 2 === 0 ? 'bg-gray-100' : 'bg-white'}`}
          >
            {item}
          </li>
        ))}
      </ul>

      {/* Complex nested conditional */}
      <div
        className={
          isActive
            ? (hasError ? 'text-red-600 font-bold' : 'text-green-600 font-medium')
            : 'text-gray-500 font-normal'
        }
      >
        Status indicator
      </div>

      {/* Spread with className override */}
      <span className="inline-block px-2 py-1 bg-blue-100" {...(hasError && { className: 'inline-block px-2 py-1 bg-red-100' })}>
        Badge
      </span>
    </div>
  );
}