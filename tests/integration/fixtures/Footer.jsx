import React from 'react';

export default function Footer() {
  return (
    <footer className="footer-section bg-gray-800 text-gray-300">
      <div className="footer-content py-8 px-4 text-center">
        <p className="copyright-text text-sm mb-2">© 2024 Test Application</p>
        <div className="social-links flex justify-center space-x-3">
          <a href="#" className="text-gray-400 hover:text-white">Twitter</a>
          <a href="#" className="text-gray-400 hover:text-white">GitHub</a>
          <a href="#" className="text-gray-400 hover:text-white">LinkedIn</a>
        </div>
      </div>
    </footer>
  );
}