var e,a=java.lang.System;Packages.br.com.sankhya.jape.EntityFacade;var t=Packages.br.com.sankhya.jape.dao.JdbcWrapper,r=Packages.br.com.sankhya.modelcore.util.EntityFacadeFactory,o=Packages.br.com.sankhya.ws.ServiceContext,n=Packages.com.google.gson.JsonArray,s=Packages.com.google.gson.JsonObject,c=Packages.com.sankhya.util.JdbcUtils,g=java.util.AbstractMap,l=function(e){var o=new s,l=null,d=null,u=a.currentTimeMillis();try{(l=r.getDWFFacade().getJdbcWrapper()).openSession();var i=l.getPreparedStatement(e),y=i.execute(),p=i.getUpdateCount();d=i.getResultSet();var m=u-a.currentTimeMillis();if(y){var v=function(e){for(var a=new n,t=e.getMetaData().getColumnCount(),r=0;e.next();){r++;for(var o=new s,c=1;c<=t;c++){var l=e.getString(c),d=e.getMetaData().getColumnLabel(c);o.has(d)?o.addProperty(d+c,l):o.addProperty(d,l)}a.add(o)}return new g.SimpleEntry(a,r)}(d);o.add("rows",v.getKey()),p=v.getValue()}return o.addProperty("rowsUpdated",p),o.addProperty("executeStatus",y),o.addProperty("queryTime",Math.abs(m)),o}catch(e){throw Error("Erro: "+e)}finally{c.closeResultSet(d),t.closeSession(l)}}(null===(e=o.getCurrent().getJsonRequestBody().get("sql"))||void 0===e?void 0:e.getAsString());o.getCurrent().setJsonResponse(l);