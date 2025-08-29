// React component fixture (simulated ReScript output)
import React from 'react';

const Button = (props) => {
  const baseClasses = "px-4 py-2 font-semibold rounded-lg transition-colors";
  const variantClasses = props.variant === 'primary' 
    ? "bg-blue-500 text-white hover:bg-blue-600" 
    : "bg-gray-200 text-gray-800 hover:bg-gray-300";
  
  return React.createElement(
    'button',
    {
      className: `${baseClasses} ${variantClasses}`,
      onClick: props.onClick,
      disabled: props.disabled,
      'aria-label': "Click me"
    },
    props.children
  );
};

// JSX version
const Card = ({ title, children }) => {
  return (
    <div className="bg-white rounded-xl shadow-lg p-6">
      <h2 className="text-2xl font-bold mb-4">{title}</h2>
      <div className="space-y-2">
        {children}
      </div>
    </div>
  );
};

// Dynamic classes
const getStatusClasses = (status) => {
  const statusMap = {
    success: "bg-green-100 text-green-800",
    warning: "bg-yellow-100 text-yellow-800",
    error: "bg-red-100 text-red-800",
    info: "bg-blue-100 text-blue-800"
  };
  return statusMap[status] || "bg-gray-100 text-gray-800";
};

export { Button, Card, getStatusClasses };