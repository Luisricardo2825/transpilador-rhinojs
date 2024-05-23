import "java.nio.file.Files";
import "java.nio.file.Path";
import "java.nio.file.Paths";
import "java.util.Arrays";
import "com.google.gson.JsonObject";
import "java.util.Collections";
import "java.util.List";
import "java.util.stream.Collectors";
import "br.com.sankhya.ws.ServiceContext";
import JString from "java.lang.String";


function log() {
    var values = new JsonObject();
    var res = new JsonObject();

    var path = java.lang.System.getProperty("org.jboss.boot.log.file");

    var filePath = Paths.get(path);
    values.addProperty("filePath", filePath.toString());
    var bytes = Files.readAllBytes(filePath);

    var lines = new JString(bytes).split(System.lineSeparator());

    list = Arrays.asList(lines);
    Collections.reverse(list);
    list = list.stream().limit(1000).collect(Collectors.toList());

    var content = JString.join(System.lineSeparator(), list);
    values.addProperty("content", content);
    res.addProperty("log", content);

    ServiceContext.getCurrent().setJsonResponse(res);
    mensagem = content;
}
function query() {
    var JdbcWrapper, EntityFacadeFactory, ServiceContext, JsonArray, JsonObject, JdbcUtils, AbstractMap, request, response, System = java.lang.System;
    Packages.br.com.sankhya.extensions.actionbutton.AcaoRotinaJava, Packages.br.com.sankhya.extensions.actionbutton.ContextoAcao, Packages.br.com.sankhya.jape.EntityFacade, JdbcWrapper = Packages.br.com.sankhya.jape.dao.JdbcWrapper, EntityFacadeFactory = Packages.br.com.sankhya.modelcore.util.EntityFacadeFactory, ServiceContext = Packages.br.com.sankhya.ws.ServiceContext, JsonArray = Packages.com.google.gson.JsonArray, Packages.com.google.gson.JsonElement, JsonObject = Packages.com.google.gson.JsonObject, JdbcUtils = Packages.com.sankhya.util.JdbcUtils, java.sql.PreparedStatement, java.sql.ResultSet, java.sql.SQLException, AbstractMap = java.util.AbstractMap, java.util.Map, java.lang.Exception, response = function (e) {
        var t, a, r, s, n = new JsonObject, o = null, c = null, g = System.currentTimeMillis();
        try {
            if ((o = EntityFacadeFactory.getDWFFacade().getJdbcWrapper()).openSession(), a = (t = o.getPreparedStatement(e)).execute(), r = t.getUpdateCount(), c = t.getResultSet(), s = g - System.currentTimeMillis(), a) {
                var i = function (e) {
                    for (var t, a = new JsonArray, r = e.getMetaData().getColumnCount(), s = 0; e.next();) {
                        s++, t = new JsonObject;
                        for (var n = 1; r >= n; n++) {
                            var o = e.getString(n), c = e.getMetaData().getColumnLabel(n);
                            t.has(c) ? t.addProperty(c + n, o) : t.addProperty(c, o + "");
                        }
                        a.add(t);
                    }
                    return new AbstractMap.SimpleEntry(a, s);
                }(c);
                n.add("rows", i.getKey()), r = i.getValue();
            }
            return n.addProperty("rowsUpdated", r), n.addProperty("executeStatus", a), n.addProperty("queryTime", Math.abs(s)), n;
        } finally {
            JdbcUtils.closeResultSet(c), JdbcWrapper.closeSession(o);
        }
    }((request = ServiceContext.getCurrent().getJsonRequestBody()).get("sql") ? request.get("sql").getAsString() : ""), ServiceContext.getCurrent().setJsonResponse(response);
}
