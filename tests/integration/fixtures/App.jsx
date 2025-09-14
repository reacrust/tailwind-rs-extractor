import React from 'react';
import Header from './Header';
import Footer from './Footer';

export default function App() {
  return (
    <div className="app-container mx-auto p-4">
      <Header />
      <main className="main-content min-h-screen bg-gray-50">
        <h1 className="text-4xl font-bold text-gray-900">Welcome</h1>
        <p className="mt-4 text-lg text-gray-600">
          This is a test application with Tailwind CSS classes.
        </p>
      </main>
      <Footer />
    </div>
  );
}