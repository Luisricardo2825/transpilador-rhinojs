import QueryExecutor from "./query/QueryExecutor";
import { ResultSerializer as Serializer } from "./query/ResultSerializer";
import * as Sql from "./query/Sql";
import { ServiceContext } from "java:br.com.sankhya.ws.ServiceContext";

const context = ServiceContext.getCurrent();
const request = context.getJsonRequestBody();
const sql = Sql.addLimit(
  request.get("sql").getAsString(),
  request.get("limit"),
);
const result = new QueryExecutor().execute(sql);

context.setJsonResponse(Serializer.toResponse(result));
