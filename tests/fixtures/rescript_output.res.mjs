// Simulated ReScript compiled output
import * as React from "react";
import * as Caml_option from "./stdlib/caml_option.js";

function Header(Props) {
  var title = Props.title;
  var subtitle = Props.subtitle;
  var tmp = {
    className: "flex items-center justify-between px-6 py-4 bg-white shadow-sm"
  };
  if (subtitle !== undefined) {
    return React.createElement("header", tmp, React.createElement("div", undefined, React.createElement("h1", {
      className: "text-3xl font-bold text-gray-900"
    }, title), React.createElement("p", {
      className: "mt-1 text-sm text-gray-500"
    }, Caml_option.valFromOption(subtitle))));
  } else {
    return React.createElement("header", tmp, React.createElement("h1", {
      className: "text-3xl font-bold text-gray-900"
    }, title));
  }
}

var make = Header;

function Navigation(Props) {
  var items = Props.items;
  var activeIndex = Props.activeIndex;
  return React.createElement("nav", {
    className: "flex space-x-4",
    role: "navigation",
    "aria-label": "Main navigation"
  }, items.map(function (item, index) {
    var isActive = index === activeIndex;
    var linkClasses = isActive 
      ? "px-3 py-2 text-sm font-medium text-blue-600 bg-blue-50 rounded-md" 
      : "px-3 py-2 text-sm font-medium text-gray-700 hover:text-gray-900 hover:bg-gray-50 rounded-md";
    return React.createElement("a", {
      key: String(index),
      href: item.href,
      className: linkClasses,
      "aria-current": isActive ? "page" : undefined
    }, item.label);
  }));
}

// Layout component
function Layout(Props) {
  var children = Props.children;
  var sidebar = Props.sidebar;
  var hasSidebar = sidebar !== undefined;
  var mainClasses = hasSidebar 
    ? "flex-1 px-8 py-6" 
    : "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6";
    
  return React.createElement("div", {
    className: "min-h-screen bg-gray-50"
  }, React.createElement("div", {
    className: hasSidebar ? "flex" : undefined
  }, hasSidebar ? React.createElement("aside", {
    className: "w-64 bg-white shadow-md"
  }, Caml_option.valFromOption(sidebar)) : null, React.createElement("main", {
    className: mainClasses
  }, children)));
}

export {
  make,
  Header,
  Navigation,
  Layout
};