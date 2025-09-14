import React from 'react';

export default function Header() {
  return (
    <header className="header-wrapper bg-blue-600 text-white">
      <nav className="navigation-bar flex justify-between items-center p-4">
        <span className="logo-text text-xl font-semibold">Logo</span>
        <ul className="nav-menu flex space-x-4">
          <li><a href="#" className="hover:text-blue-200">Home</a></li>
          <li><a href="#" className="hover:text-blue-200">About</a></li>
          <li><a href="#" className="hover:text-blue-200">Contact</a></li>
        </ul>
      </nav>
    </header>
  );
}