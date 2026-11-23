import { Navigate, Route, Routes, useLocation } from "react-router";
import { Layout } from "./components/layout";
import { Hub } from "./modules/Hub";
import { Home } from "./modules/Home";
import { Information } from "./modules/extras/infrastructure";
import { WidgetView } from "./modules/resources/Widget/View";
import { pageRoutes, sections, widgetGroups, widgetRoute } from "./modules/Hub/catalog";

function ExistingEntry() {
  const { pathname, search } = useLocation();
  const params = new URLSearchParams(search);
  const id = params.get("id") || "";
  if (pathname === "/widget" && !widgetGroups[id]) return <WidgetView />;
  const destination = pathname === "/widget"
    ? widgetRoute(id)
    : pathname === "/wallpaper"
    ? "/appearance?section=wallpapers"
    : pageRoutes[pathname] || "/appearance";
  const [path, query] = destination.split("?");
  const next = new URLSearchParams(query);
  params.forEach((value, key) => {
    if (key !== "id") next.set(key, value);
  });
  if (pathname === "/wallpaper" && id) next.set("wallpaper", id);
  return <Navigate to={`${path}?${next}`} replace />;
}

export function Routing() {
  return (
    <Routes>
      <Route Component={Layout}>
        <Route index Component={Home} />
        {sections.map(({ route }) => <Route key={route} path={route} Component={Hub} />)}
        <Route path="extras" Component={Information} />
        {Object.keys(pageRoutes).map((path) => <Route key={path} path={path} Component={ExistingEntry} />)}
        <Route path="widget" Component={ExistingEntry} />
        <Route path="wallpaper" Component={ExistingEntry} />
        <Route path="theme" element={<Navigate to="/appearance" replace />} />
        <Route path="resources/*" element={<Navigate to="/appearance?section=wallpapers" replace />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  );
}
