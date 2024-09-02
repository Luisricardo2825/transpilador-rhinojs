import "br.com.sankhya.jape.EntityFacade";
import "br.com.sankhya.jape.dao.JdbcWrapper";
import "br.com.sankhya.modelcore.util.EntityFacadeFactory";
import "br.com.sankhya.ws.ServiceContext";
import { JsonArray } from "com.google.gson";
import "com.google.gson.JsonObject";
import "com.sankhya.util.JdbcUtils";
import "java.nio.file.Files";
import "java.nio.file.Path";
import "java.nio.file.Paths";
import { AbstractMap } from "java.util";
import "java.util.Arrays";
import "java.util.Collections";
import "java.util.List";
import "java.util.stream.Collectors";

function query() {
    function execute(query) {
        const response = new JsonObject();
        var jdbc = null;
        var rset = null;
        var start = System.currentTimeMillis();
        try {
            const entity = EntityFacadeFactory.getDWFFacade();
            jdbc = entity.getJdbcWrapper();
            jdbc.openSession();
            var upd = jdbc.getPreparedStatement(query);
            var executeStatus = upd.execute();
            var rowsUpdated = upd.getUpdateCount();
            rset = upd.getResultSet();
            var end = start - System.currentTimeMillis();
            if (executeStatus) {
                var rows = GetResults(rset);
                response.add("rows", rows.getKey());
                rowsUpdated = rows.getValue();
            }
            response.addProperty("rowsUpdated", rowsUpdated);
            response.addProperty("executeStatus", executeStatus);
            response.addProperty("queryTime", Math.abs(end));
            return response;
        } catch (e) {
            throw new Error("Erro: " + e);
        } finally {
            JdbcUtils.closeResultSet(rset);
            JdbcWrapper.closeSession(jdbc);
        }
    }

    function GetResults(rset) {
        var results = new JsonArray();
        var total_cols = rset.getMetaData().getColumnCount();
        var rowsUpdated = 0;


        while (rset.next()) {
            rowsUpdated++;
            var colJson = new JsonObject();
            for (var col = 1; col <= total_cols; col++) {
                var value = rset.getString(col);
                var colLabel = rset.getMetaData().getColumnLabel(col);
                if (!colJson.has(colLabel)) {
                    colJson.addProperty(colLabel, value);
                } else {
                    colJson.addProperty(colLabel + col, value);
                }
            }
            results.add(colJson);
        }
        return new AbstractMap.SimpleEntry(results, rowsUpdated);
    }

    const ctx = ServiceContext.getCurrent();
    const request = ctx.getJsonRequestBody();
    const sql = request.get("sql")?.getAsString();
    const response = execute(sql);
    ServiceContext.getCurrent().setJsonResponse(response);
}

message = query()
